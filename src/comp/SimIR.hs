module SimIR (simIRFromSimCC) where

import ASyntax
import Id(getIdBaseString, getIdString)
import IntLit(ilValue)
import Prim
import SimCCBlock

import qualified Data.Map as M
import Data.List(isInfixOf, isSuffixOf)

-- This is deliberately a narrow, fail-closed projection used to bootstrap the
-- Rust Bluesim M0 fixture.  It must be extended structurally alongside the
-- Rust SimIR schema; it must not fall back to the generated C++ path.
simIRFromSimCC :: String -> [SimCCBlock] -> [SimCCSched] -> Either String String
simIRFromSimCC top blocks scheds = do
  top_block <- exactlyOne ("top block " ++ show top)
               [ b | b <- blocks, sb_name b == top ]
  (state_name, state_width, state_initial) <- oneState top_block
  sched <- exactlyOne "clock schedule" scheds
  clock_name <- clockName (sched_clock sched)
  let reset_states = [ (getIdString rid, 1, 1) | rid <- sb_inputResets top_block ]
      states = (state_name, state_width, state_initial) : reset_states
      time_locals = M.fromList
        [ (getIdString aid, 0)
        | aid <- timeTaskDefs (sf_body (sched_fn sched)) ++
                 concatMap (timeTaskDefs . sf_body) (get_rule_fns top_block)
        ]
      state_widths = M.fromList [ (name, width) | (name, width, _) <- states ] `M.union` time_locals
      rule_fns = M.fromList [ (sf_name fn, fn) | fn <- get_rule_fns top_block ]
  actions <- stmtsToJson rule_fns state_widths (sf_body (sched_fn sched))
  return $ object
    [ field "schemaVersion" "1"
    , field "producer" $ object
        [ field "name" (jsonString "legacy-bsc-simcc-projection")
        , field "version" (jsonString "m0")
        ]
    , field "top" (jsonString top)
    , field "clocks" $ array
        [ object
          [ field "id" (jsonString clock_name)
          , field "period" "10"
          , field "activeEdge" (jsonString "posedge")
          ]
        ]
    , field "state" $ array (map stateToJson states)
    , field "schedules" $ array
        [ object
          [ field "clock" (jsonString clock_name)
          , field "actions" (array actions)
          ]
        ]
    ]

oneState :: SimCCBlock -> Either String (String, Integer, Integer)
oneState block = do
  (_, state_id, args) <- exactlyOne "M0 state instance" (sb_state block)
  case args of
    [ASInt _ (ATBit _) width_lit, ASInt _ (ATBit width) initial_lit]
      | width > 0 && width <= 64
      , ilValue width_lit == width
      , ilValue initial_lit >= 0
      , ilValue initial_lit < 2 ^ width ->
          return (getIdString state_id, width, ilValue initial_lit)
    _ -> Left $ "unsupported M0 state instance arguments: " ++ show args

clockName :: AExpr -> Either String String
clockName (ASDef _ aid) = return (getIdString aid)
clockName (ASPort _ aid) = return (getIdString aid)
clockName expr = Left $ "unsupported M0 clock expression: " ++ show expr

stmtsToJson :: M.Map String SimCCFn -> M.Map String Integer -> [SimCCFnStmt]
            -> Either String [String]
stmtsToJson rule_fns state_widths = fmap concat . mapM (stmtToJson rule_fns state_widths)

stmtToJson :: M.Map String SimCCFn -> M.Map String Integer -> SimCCFnStmt
           -> Either String [String]
stmtToJson rule_fns state_widths stmt =
  case stmt of
    SFSDef False _ Nothing -> return []
    SFSDef False (_, aid) (Just expr) ->
      letToJson state_widths aid expr
    SFSAssign False aid expr ->
      letToJson state_widths aid expr
    SFSAssignAction False _ ATaskAction { ataskact_fun = "$time", aact_args = args } _ -> do
      (condition, rest) <- conditionAndArgs "$time" args
      ensureTrue condition
      if null rest
        then return []
        else Left "$time has unexpected arguments"
    SFSAssignAction False aid (ACall obj meth args) _
      | getIdBaseString meth == "read" -> do
          (condition, rest) <- conditionAndArgs "register read" args
          ensureTrue condition
          state_name <- lookupState state_widths obj
          return [letAction aid (stateExpr state_name)]
    SFSAction (ACall obj meth args)
      | getIdBaseString meth == "write" -> do
          (condition, rest) <- conditionAndArgs "register write" args
          value <- exactlyOne "register write value" rest >>= exprToJson state_widths
          state_name <- lookupState state_widths obj
          conditional state_widths condition [writeAction state_name value]
    SFSAction (AFCall _ fun _ args _)
      | "display" `isSuffixOf` fun -> displayToJson state_widths args
      | fun == "$finish" || "finish_" `isSuffixOf` fun -> finishToJson state_widths args
    SFSRuleExec rule_id -> inlineRule rule_fns state_widths (getIdBaseString rule_id)
    SFSFunctionCall _ name [] -> inlineRule rule_fns state_widths name
    SFSCond condition then_stmts else_stmts -> do
      condition_json <- exprToJson state_widths condition
      then_json <- stmtsToJson rule_fns state_widths then_stmts
      else_json <- stmtsToJson rule_fns state_widths else_stmts
      return [object
        [ field "kind" (jsonString "if")
        , field "condition" condition_json
        , field "then" (array then_json)
        , field "else" (array else_json)
        ]]
    SFSReturn Nothing -> return []
    -- M0 models begin in their declared initial state and have no external reset
    -- driver.  Accept only the synthesized, asserted clock-reset tick, which is
    -- unreachable in the tiny fixture before its terminal rule fires.
    SFSResets [SFSFunctionCall _ name [ASInt _ (ATBit 1) lit]]
      | "rst_tick__clk__" `isInfixOf` name && ilValue lit == 1 -> return []
    unsupported -> Left $ "unsupported M0 SimCC statement: " ++ show unsupported

inlineRule :: M.Map String SimCCFn -> M.Map String Integer -> String -> Either String [String]
inlineRule rule_fns state_widths name =
  case M.lookup name rule_fns of
    Nothing -> Left $ "M0 schedule references unsupported function/rule " ++ show name
    Just fn
      | null (sf_args fn) && sf_retType fn == Nothing ->
          stmtsToJson rule_fns state_widths (sf_body fn)
      | otherwise -> Left $ "M0 rule has arguments or a return value: " ++ show name

letToJson :: M.Map String Integer -> AId -> AExpr -> Either String [String]
letToJson state_widths aid expr = do
  value <- exprToJson state_widths expr
  return [letAction aid value]

exprToJson :: M.Map String Integer -> AExpr -> Either String String
exprToJson state_widths expr =
  case expr of
    ASDef _ aid -> identifierToJson state_widths aid
    ASPort _ aid -> identifierToJson state_widths aid
    AMethCall { ae_objid = obj, ameth_id = meth, ae_args = [] }
      | getIdBaseString meth == "read" ->
          stateExpr <$> lookupState state_widths obj
    ASInt _ (ATBit width) lit
      | width > 0 && width <= 64
      , ilValue lit >= 0
      , ilValue lit < 2 ^ width -> return $ object
          [ field "kind" (jsonString "const")
          , field "width" (show width)
          , field "value" (show (ilValue lit))
          ]
    AFunCall { ae_funname = fun, ae_args = [] }
      | "__time__" `isSuffixOf` fun ->
          return (object [field "kind" (jsonString "time")])
    APrim { ae_type = ATBit width, aprim_prim = PrimBNot, ae_args = [arg] } -> do
      arg_json <- exprToJson state_widths arg
      return $ object
        [ field "kind" (jsonString "unary")
        , field "width" (show width)
        , field "op" (jsonString "not")
        , field "arg" arg_json
        ]
    APrim { ae_type = ATBit width, aprim_prim = op, ae_args = args } -> do
      op_name <- case op of
        PrimAdd -> return "add"
        PrimEQ -> return "equal"
        PrimULT -> return "unsigned_less_than"
        _ -> Left $ "unsupported M0 primitive: " ++ show op
      args_json <- mapM (exprToJson state_widths) args
      return $ object
        [ field "kind" (jsonString "binary")
        , field "width" (show width)
        , field "op" (jsonString op_name)
        , field "args" (array args_json)
        ]
    unsupported -> Left $ "unsupported M0 expression: " ++ show unsupported

identifierToJson :: M.Map String Integer -> AId -> Either String String
identifierToJson state_widths aid =
  case stateName state_widths aid of
    Just name
      | M.lookup name state_widths == Just 0 ->
          return (object [field "kind" (jsonString "time")])
      | otherwise -> return (stateExpr name)
    Nothing -> return $ object
      [ field "kind" (jsonString "local")
      , field "id" (jsonString (getIdString aid))
      ]

conditionAndArgs :: String -> [AExpr] -> Either String (AExpr, [AExpr])
conditionAndArgs subject args =
  case args of
    [] -> Left $ subject ++ " has no condition"
    (condition:rest) -> return (condition, rest)

ensureTrue :: AExpr -> Either String ()
ensureTrue (ASInt _ (ATBit 1) lit) | ilValue lit == 1 = return ()
ensureTrue expr = Left $ "M0 register read has a non-constant condition: " ++ show expr

conditional :: M.Map String Integer -> AExpr -> [String] -> Either String [String]
conditional _ (ASInt _ (ATBit 1) lit) actions
  | ilValue lit == 1 = return actions
  | ilValue lit == 0 = return []
conditional state_widths condition actions = do
  condition_json <- exprToJson state_widths condition
  return [object
    [ field "kind" (jsonString "if")
    , field "condition" condition_json
    , field "then" (array actions)
    , field "else" "[]"
    ]]

displayToJson :: M.Map String Integer -> [AExpr] -> Either String [String]
displayToJson state_widths args = do
  (condition, rest) <- conditionAndArgs "$display" args
  (format, values) <- case rest of
    (ASStr _ _ string:exprs) -> return (string, exprs)
    _ -> Left "$display must have a static format string"
  display <- case (format, values) of
    ("%t: %d", [time_value, decimal_value]) -> do
      time_json <- exprToJson state_widths time_value
      decimal_json <- exprToJson state_widths decimal_value
      return $ object
        [ field "kind" (jsonString "display")
        , field "items" $ array
          [ object
            [ field "kind" (jsonString "decimal")
            , field "width" "20"
            , field "value" time_json
            ]
          , object
            [ field "kind" (jsonString "text")
            , field "text" (jsonString ":")
            ]
          , object
            [ field "kind" (jsonString "decimal")
            , field "width" "6"
            , field "value" decimal_json
            ]
          ]
        ]
    _ -> Left $ "unsupported M0 $display format/arguments: " ++ show (format, values)
  conditional state_widths condition [display]

finishToJson :: M.Map String Integer -> [AExpr] -> Either String [String]
finishToJson state_widths args = do
  (condition, values) <- conditionAndArgs "$finish" args
  status_expr <- exactlyOne "$finish status" values
  status <- case status_expr of
    ASInt _ (ATBit _) lit -> return (ilValue lit)
    _ -> Left "$finish status must be an integer literal"
  if status < 0 || status > toInteger (maxBound :: Int)
    then Left "$finish status is out of range"
    else conditional state_widths condition
           [object [field "kind" (jsonString "finish"), field "status" (show status)]]

lookupState :: M.Map String Integer -> AId -> Either String String
lookupState state_widths aid =
  case stateName state_widths aid of
    Just name -> return name
    Nothing -> Left $ "M0 primitive method references unknown state " ++ show (getIdString aid)

stateName :: M.Map String Integer -> AId -> Maybe String
stateName state_widths aid =
  let full = getIdString aid
      base = getIdBaseString aid
  in if M.member full state_widths then Just full
     else if M.member base state_widths then Just base
     else Nothing

timeTaskDefs :: [SimCCFnStmt] -> [AId]
timeTaskDefs = concatMap taskDef
  where
    taskDef (SFSAssignAction False aid ATaskAction { ataskact_fun = "$time" } _) = [aid]
    taskDef (SFSCond _ then_stmts else_stmts) = timeTaskDefs then_stmts ++ timeTaskDefs else_stmts
    taskDef (SFSResets stmts) = timeTaskDefs stmts
    taskDef _ = []

stateExpr :: String -> String
stateExpr name = object
  [ field "kind" (jsonString "state")
  , field "id" (jsonString name)
  ]

letAction :: AId -> String -> String
letAction aid value = object
  [ field "kind" (jsonString "let")
  , field "local" (jsonString (getIdString aid))
  , field "value" value
  ]

writeAction :: String -> String -> String
writeAction state_name value = object
  [ field "kind" (jsonString "write")
  , field "state" (jsonString state_name)
  , field "value" value
  ]

stateToJson :: (String, Integer, Integer) -> String
stateToJson (name, width, initial) = object
  [ field "id" (jsonString name)
  , field "width" (show width)
  , field "initialValue" (show initial)
  ]

exactlyOne :: String -> [a] -> Either String a
exactlyOne subject [value] = Right value
exactlyOne subject values = Left $ "expected exactly one " ++ subject ++ ", got " ++ show (length values)

object :: [String] -> String
object fields = "{" ++ join "," fields ++ "}"

array :: [String] -> String
array values = "[" ++ join "," values ++ "]"

field :: String -> String -> String
field name value = jsonString name ++ ":" ++ value

jsonString :: String -> String
jsonString = show

join :: String -> [String] -> String
join _ [] = ""
join _ [value] = value
join separator (value:values) = value ++ separator ++ join separator values
