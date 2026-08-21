module SimIR (simIRFromSimCC) where

import ASyntax
import Id(getIdBaseString, getIdString)
import IntLit(ilValue)
import Prim
import SimCCBlock

import qualified Data.Map as M
import Data.List(isInfixOf, isPrefixOf, isSuffixOf)

-- This is deliberately a narrow, fail-closed projection used to bootstrap the
-- Rust Bluesim M0 fixture.  It must be extended structurally alongside the
-- Rust SimIR schema; it must not fall back to the generated C++ path.
simIRFromSimCC :: String -> [SimCCBlock] -> [SimCCSched] -> Either String String
simIRFromSimCC top blocks scheds = do
  top_block <- exactlyOne ("top block " ++ show top)
               [ b | b <- blocks, sb_name b == top ]
  if any (`elem` ["ClockGen", "InitialReset"]) (primitiveNames top_block)
    then simIRM2FromSimCC top top_block scheds
    else if any (maybe True (const False) . primitiveName . fst3) (sb_state top_block)
      then simIRM3FromSimCC top blocks top_block scheds
      else simIRM0FromSimCC top top_block scheds

-- Keep the established M0 projection byte-for-byte unchanged for its tiny
-- single-clock shape.  M2 is selected only when the top instantiates one of
-- the two primitives that define the deliberately narrow multi-clock shape.
simIRM0FromSimCC :: String -> SimCCBlock -> [SimCCSched] -> Either String String
simIRM0FromSimCC top top_block scheds = do
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

fst3 :: (a, b, c) -> a
fst3 (value, _, _) = value

-- M3 is a deliberately closed single-clock projection for one hierarchical
-- module tree.  It flattens only RegN state, then inlines the already-ordered
-- SimCC schedule, its rule calls, and side-effect-free return methods.  This
-- is semantic SimIR, not a C++ class layout or a method-dispatch ABI.
data M3Context = M3Context
  { m3_blocks :: M.Map String SimCCBlock
  , m3_children :: M.Map String (M.Map String String)
  , m3_state_widths :: M.Map String Integer
  }

simIRM3FromSimCC :: String -> [SimCCBlock] -> SimCCBlock -> [SimCCSched]
                 -> Either String String
simIRM3FromSimCC top blocks top_block scheds = do
  sched <- exactlyOne "M3 clock schedule" scheds
  ensure (sched_posedge sched) "M3 supports only posedge schedules"
  ensure (sched_after_fn sched == Nothing) "M3 does not support after-edge schedules"
  clock <- clockName (sched_clock sched)
  ensure (clock == "CLK") "M3 supports only the default CLK clock"
  let by_id = M.fromList [ (sb_id block, block) | block <- blocks ]
      scoped_blocks = m3ScopedBlocks by_id "" top_block
      child_scopes = m3ChildScopes by_id "" top_block
  states <- m3FlattenStates by_id "" top_block
  ensure (not (null states)) "M3 requires at least one RegN state"
  let context = M3Context scoped_blocks child_scopes
                (M.fromList [ (name, width) | (name, width, _) <- states ])
  actions <- m3StmtsToJson context "" M.empty (sf_body (sched_fn sched))
  return $ object
    [ field "schemaVersion" "3"
    , field "producer" $ object
        [ field "name" (jsonString "legacy-bsc-simcc-projection")
        , field "version" (jsonString "m3")
        ]
    , field "top" (jsonString top)
    , field "clocks" $ array
        [ object
          [ field "id" (jsonString clock)
          , field "period" "10"
          , field "activeEdge" (jsonString "posedge")
          ]
        ]
    , field "state" $ array (map stateToJson states)
    , field "schedules" $ array
        [ object
          [ field "clock" (jsonString clock)
          , field "actions" (array actions)
          ]
        ]
    ]

m3ScopedBlocks :: M.Map SBId SimCCBlock -> String -> SimCCBlock -> M.Map String SimCCBlock
m3ScopedBlocks by_id scope block =
  M.insert scope block $ M.unions
    [ m3ScopedBlocks by_id (m3Scope scope (getIdBaseString instance_id)) child
    | (block_id, instance_id, _) <- sb_state block
    , Nothing <- [primitiveName block_id]
    , Just child <- [M.lookup block_id by_id]
    ]

m3ChildScopes :: M.Map SBId SimCCBlock -> String -> SimCCBlock -> M.Map String (M.Map String String)
m3ChildScopes by_id scope block =
  M.insert scope direct $ M.unions
    [ m3ChildScopes by_id child_scope child
    | (block_id, instance_id, _) <- sb_state block
    , Nothing <- [primitiveName block_id]
    , Just child <- [M.lookup block_id by_id]
    , let child_scope = m3Scope scope (getIdBaseString instance_id)
    ]
  where
    direct = M.fromList
      [ (getIdBaseString instance_id, m3Scope scope (getIdBaseString instance_id))
      | (block_id, instance_id, _) <- sb_state block
      , Nothing <- [primitiveName block_id]
      ]

m3FlattenStates :: M.Map SBId SimCCBlock -> String -> SimCCBlock
                -> Either String [(String, Integer, Integer)]
m3FlattenStates by_id scope block = do
  child_states <- fmap concat $ mapM flatten (sb_state block)
  let reset_states =
        [ (m3Scope scope (getIdBaseString reset_id), 1, 1)
        | reset_id <- sb_inputResets block
        ]
  return (child_states ++ reset_states)
  where
    flatten (block_id, instance_id, args) =
      case primitiveName block_id of
        Just "RegN" -> do
          (_, width, initial) <- m2RegState (instance_id, args)
          return [(m3Scope scope (getIdBaseString instance_id), width, initial)]
        Just name -> Left $ "M3 supports only RegN primitives, found " ++ name
        Nothing ->
          case M.lookup block_id by_id of
            Nothing -> Left $ "M3 contains an unknown submodule block " ++ show block_id
            Just child -> m3FlattenStates by_id (m3Scope scope (getIdBaseString instance_id)) child

m3Scope :: String -> String -> String
m3Scope "" name = name
m3Scope scope name = scope ++ "." ++ name

m3StmtsToJson :: M3Context -> String -> M.Map String String -> [SimCCFnStmt]
              -> Either String [String]
m3StmtsToJson context scope bindings = fmap concat . mapM (m3StmtToJson context scope bindings)

m3StmtToJson :: M3Context -> String -> M.Map String String -> SimCCFnStmt
             -> Either String [String]
m3StmtToJson context scope bindings stmt =
  case stmt of
    SFSDef False _ Nothing -> return []
    SFSDef False (_, aid) (Just expr) -> m3LetToJson context scope bindings aid expr
    SFSAssign False aid expr -> m3LetToJson context scope bindings aid expr
    SFSAssignAction False aid (ACall obj meth args) _
      | getIdBaseString meth == "read" -> do
          (condition, rest) <- conditionAndArgs "M3 register read" args
          ensureTrue condition
          ensure (null rest) "M3 register read has unexpected arguments"
          state_name <- m3StateFor context scope obj
          return [m3LetAction (m3LocalId scope aid) (stateExpr state_name)]
    SFSAction (ACall obj meth args)
      | getIdBaseString meth == "write" -> do
          (condition, rest) <- conditionAndArgs "M3 register write" args
          value <- exactlyOne "M3 register write value" rest >>= m3ExprToJson context scope bindings
          state_name <- m3StateFor context scope obj
          m3Conditional context scope bindings condition [writeAction state_name value]
      | otherwise -> do
          (condition, method_args) <- conditionAndArgs "M3 action method" args
          target_scope <- m3CallScope context scope obj
          actions <- m3InlineFunction context scope target_scope bindings (getIdBaseString meth) method_args
          m3Conditional context scope bindings condition actions
    SFSAction (AFCall _ fun _ args _)
      | fun == "$finish" || "finish_" `isSuffixOf` fun -> m3FinishToJson context scope bindings args
    SFSRuleExec rule_id -> m3InlineFunction context scope scope bindings (getIdBaseString rule_id) []
    SFSFunctionCall _ name [arg]
      | "rst_tick__clk__" `isInfixOf` name && isOneBit arg -> return []
    SFSFunctionCall object "reset_RST" [arg]
      | isOneBit arg -> do
          _ <- m3StateFor context scope object
          return []
    SFSFunctionCall object name args -> do
      target_scope <-
        case m3CallScope context scope object of
          Left err -> Left $ err ++ " for function " ++ show name ++ " with arguments " ++ show args
          Right value -> Right value
      m3InlineFunction context scope target_scope bindings name args
    SFSCond condition then_stmts else_stmts -> do
      condition_json <- m3ExprToJson context scope bindings condition
      then_json <- m3StmtsToJson context scope bindings then_stmts
      else_json <- m3StmtsToJson context scope bindings else_stmts
      return [object
        [ field "kind" (jsonString "if")
        , field "condition" condition_json
        , field "then" (array then_json)
        , field "else" (array else_json)
        ]]
    SFSReturn Nothing -> return []
    SFSResets reset_stmts
      | all m3ResetTick reset_stmts -> return []
    unsupported -> Left $ "unsupported M3 SimCC statement: " ++ show unsupported

m3ResetTick :: SimCCFnStmt -> Bool
m3ResetTick (SFSFunctionCall _ name [arg]) = "rst_tick__clk__" `isInfixOf` name && isOneBit arg
m3ResetTick _ = False

m3InlineFunction :: M3Context -> String -> String -> M.Map String String -> String -> [AExpr]
                 -> Either String [String]
m3InlineFunction context caller_scope target_scope caller_bindings name args = do
  block <- m3Block context target_scope
  let local_matches =
        [ (target_scope, candidate)
        | candidate <- get_rule_fns block ++ get_method_fns block
        , sf_name candidate == name
        ]
      global_matches =
        [ (candidate_scope, candidate)
        | (candidate_scope, candidate_block) <- M.toList (m3_blocks context)
        , candidate <- get_rule_fns candidate_block ++ get_method_fns candidate_block
        , sf_name candidate == name
        ]
  (effective_scope, fn) <- exactlyOne ("M3 function " ++ name)
                           (if null local_matches then global_matches else local_matches)
  ensure (sf_retType fn == Nothing) ("M3 statement call returns a value: " ++ name)
  bindings <- m3FunctionBindings context caller_scope caller_bindings fn args
  m3StmtsToJson context effective_scope bindings (sf_body fn)

m3FunctionBindings :: M3Context -> String -> M.Map String String -> SimCCFn -> [AExpr]
                   -> Either String (M.Map String String)
m3FunctionBindings context scope caller_bindings fn args = do
  ensure (length (sf_args fn) == length args) ("M3 function has wrong argument count: " ++ sf_name fn)
  values <- mapM (m3ExprToJson context scope caller_bindings) args
  return $ M.fromList
    [ (getIdString argument, value)
    | ((_, argument), value) <- zip (sf_args fn) values
    ]

m3LetToJson :: M3Context -> String -> M.Map String String -> AId -> AExpr
            -> Either String [String]
m3LetToJson context scope bindings aid expr = do
  value <- m3ExprToJson context scope bindings expr
  return [m3LetAction (m3LocalId scope aid) value]

m3LetAction :: String -> String -> String
m3LetAction name value = object
  [ field "kind" (jsonString "let")
  , field "local" (jsonString name)
  , field "value" value
  ]

m3ExprToJson :: M3Context -> String -> M.Map String String -> AExpr -> Either String String
m3ExprToJson context scope bindings expr =
  case expr of
    ASDef _ aid -> m3IdentifierToJson context scope bindings aid
    ASPort _ aid -> m3IdentifierToJson context scope bindings aid
    AMethCall { ae_objid = obj, ameth_id = meth, ae_args = [] }
      | getIdBaseString meth == "read" -> stateExpr <$> m3StateFor context scope obj
      | otherwise -> m3MethodExpr context scope bindings obj (getIdBaseString meth)
    ASInt _ (ATBit width) lit
      | width > 0 && width <= 64
      , ilValue lit >= 0
      , ilValue lit < 2 ^ width -> return $ object
          [ field "kind" (jsonString "const")
          , field "width" (show width)
          , field "value" (show (ilValue lit))
          ]
    APrim { ae_type = ATBit width, aprim_prim = PrimBNot, ae_args = [arg] } -> do
      arg_json <- m3ExprToJson context scope bindings arg
      return $ object
        [ field "kind" (jsonString "unary")
        , field "width" (show width)
        , field "op" (jsonString "not")
        , field "arg" arg_json
        ]
    APrim { ae_type = ATBit width, aprim_prim = PrimULE, ae_args = [left, right] }
      | width == 1 -> do
          left_json <- m3ExprToJson context scope bindings left
          right_json <- m3ExprToJson context scope bindings right
          return $ object
            [ field "kind" (jsonString "unary")
            , field "width" "1"
            , field "op" (jsonString "not")
            , field "arg" $ object
                [ field "kind" (jsonString "binary")
                , field "width" "1"
                , field "op" (jsonString "unsigned_less_than")
                , field "args" (array [right_json, left_json])
                ]
            ]
    APrim { ae_type = ATBit width, aprim_prim = op, ae_args = args } -> do
      op_name <- case op of
        PrimAdd -> return "add"
        PrimBAnd -> return "and"
        PrimSub -> return "sub"
        PrimEQ -> return "equal"
        PrimULT -> return "unsigned_less_than"
        _ -> Left $ "unsupported M3 primitive: " ++ show op
      args_json <- mapM (m3ExprToJson context scope bindings) args
      return $ object
        [ field "kind" (jsonString "binary")
        , field "width" (show width)
        , field "op" (jsonString op_name)
        , field "args" (array args_json)
        ]
    unsupported -> Left $ "unsupported M3 expression: " ++ show unsupported

m3MethodExpr :: M3Context -> String -> M.Map String String -> AId -> String -> Either String String
m3MethodExpr context scope caller_bindings object name = do
  target_scope <- m3CallScope context scope object
  block <- m3Block context target_scope
  fn <- exactlyOne ("M3 return method " ++ name)
        [ candidate | candidate <- get_method_fns block, sf_name candidate == name ]
  ensure (null (sf_args fn)) ("M3 return method has arguments: " ++ name)
  ensure (sf_retType fn /= Nothing) ("M3 return method has no return value: " ++ name)
  m3PureMethodExpr context target_scope caller_bindings (sf_body fn)

m3PureMethodExpr :: M3Context -> String -> M.Map String String -> [SimCCFnStmt]
                 -> Either String String
m3PureMethodExpr context scope bindings stmts = go bindings stmts
  where
    go _ [] = Left "M3 return method has no return"
    go local_bindings (SFSDef _ _ Nothing:rest) = go local_bindings rest
    go local_bindings (SFSDef _ (_, aid) (Just expr):rest) = do
      value <- m3ExprToJson context scope local_bindings expr
      go (M.insert (getIdString aid) value local_bindings) rest
    go local_bindings (SFSAssign _ aid expr:rest) = do
      value <- m3ExprToJson context scope local_bindings expr
      go (M.insert (getIdString aid) value local_bindings) rest
    go local_bindings (SFSReturn (Just expr):[]) =
      m3ExprToJson context scope local_bindings expr
    go _ unsupported = Left $ "unsupported M3 return method body: " ++ show unsupported

m3IdentifierToJson :: M3Context -> String -> M.Map String String -> AId -> Either String String
m3IdentifierToJson context scope bindings aid =
  case M.lookup (getIdString aid) bindings of
    Just value -> return value
    Nothing ->
      case m3MaybeStateFor context scope aid of
        Just state_name -> return (stateExpr state_name)
        Nothing -> return $ object
          [ field "kind" (jsonString "local")
          , field "id" (jsonString (m3LocalId scope aid))
          ]

m3StateFor :: M3Context -> String -> AId -> Either String String
m3StateFor context scope aid =
  case m3MaybeStateFor context scope aid of
    Just name -> return name
    Nothing -> Left $ "M3 primitive method references unknown state " ++ show (getIdString aid)

m3MaybeStateFor :: M3Context -> String -> AId -> Maybe String
m3MaybeStateFor context scope aid =
  let raw = m3StripTop (getIdString aid)
      relative = m3Scope scope (getIdBaseString aid)
      candidates = [raw, relative]
  in case filter (`M.member` m3_state_widths context) candidates of
       name:_ -> Just name
       [] -> Nothing

m3CallScope :: M3Context -> String -> AId -> Either String String
m3CallScope context scope object =
  let raw = m3StripTop (getIdString object)
      self = raw == "" || raw == "top" || raw == scope
      child = M.lookup scope (m3_children context) >>= M.lookup (getIdBaseString object)
  in if self then return scope
     else case child of
       Just target -> return target
       Nothing -> Left $ "M3 call references unknown module " ++ show (getIdString object)

m3Block :: M3Context -> String -> Either String SimCCBlock
m3Block context scope =
  case M.lookup scope (m3_blocks context) of
    Just block -> return block
    Nothing -> Left $ "M3 references unknown module scope " ++ show scope

m3LocalId :: String -> AId -> String
m3LocalId scope aid = m3Scope scope (getIdString aid)

m3StripTop :: String -> String
m3StripTop name
  | "top." `isPrefixOf` name = drop 4 name
  | name == "top" = ""
  | otherwise = name

m3Conditional :: M3Context -> String -> M.Map String String -> AExpr -> [String]
              -> Either String [String]
m3Conditional _ _ _ (ASInt _ (ATBit 1) lit) actions
  | ilValue lit == 1 = return actions
  | ilValue lit == 0 = return []
m3Conditional context scope bindings condition actions = do
  condition_json <- m3ExprToJson context scope bindings condition
  return [object
    [ field "kind" (jsonString "if")
    , field "condition" condition_json
    , field "then" (array actions)
    , field "else" "[]"
    ]]

m3FinishToJson :: M3Context -> String -> M.Map String String -> [AExpr]
               -> Either String [String]
m3FinishToJson context scope bindings args = do
  (condition, values) <- conditionAndArgs "M3 $finish" args
  status_expr <- exactlyOne "M3 $finish status" values
  status <- case status_expr of
    ASInt _ (ATBit _) lit -> return (ilValue lit)
    _ -> Left "M3 $finish status must be an integer literal"
  ensure (status >= 0 && status <= toInteger (maxBound :: Int)) "M3 $finish status is out of range"
  m3Conditional context scope bindings condition
    [object [field "kind" (jsonString "finish"), field "status" (show status)]]

primitiveNames :: SimCCBlock -> [String]
primitiveNames block =
  [ name
  | (block_id, _, _) <- sb_state block
  , Just name <- [primitiveName block_id]
  ]

-- The primitive list is the source of truth for both identity and name.  In
-- particular, this does not encode the unstable SBId assigned to a primitive.
primitiveName :: SBId -> Maybe String
primitiveName block_id =
  case [ sb_name block | block <- primBlocks, sb_id block == block_id ] of
    [name] -> Just name
    _ -> Nothing

data M2Reset = M2Reset String String String String String Integer

simIRM2FromSimCC :: String -> SimCCBlock -> [SimCCSched] -> Either String String
simIRM2FromSimCC top top_block scheds = do
  primitives <- m2PrimitiveInstances top_block
  ensure (all (\(name, _, _) -> name `elem` ["RegN", "ClockGen", "InitialReset"]) primitives)
         "M2 supports only RegN, ClockGen, and InitialReset primitives"
  (clockgen_id, clockgen_args) <- exactlyOne "M2 ClockGen primitive"
                                 [ (instance_id, args)
                                 | ("ClockGen", instance_id, args) <- primitives
                                 ]
  (reset_id, reset_args) <- exactlyOne "M2 InitialReset primitive"
                            [ (instance_id, args)
                            | ("InitialReset", instance_id, args) <- primitives
                            ]
  reg_states <- mapM m2RegState
                [ (instance_id, args)
                | ("RegN", instance_id, args) <- primitives
                ]
  ensure (not (null reg_states)) "M2 requires at least one RegN state"
  ensure (length primitives == length reg_states + 2)
         "M2 contains an unsupported primitive instance"
  (v1, v2, delay, initial, _) <- m2ClockGenArgs clockgen_args
  cycles <- m2InitialResetArgs reset_args
  let default_clock = "CLK"
      generated_clock = getIdString clockgen_id ++ "$CLK_OUT"
  (default_sched, generated_sched) <- m2Schedules default_clock generated_clock scheds
  reset <- m2ResetDefinition top_block reset_id generated_clock reg_states
  let M2Reset reset_name _ _ _ _ _ = reset
      reset_state = (m2ResetSignal reset, 1, 0)
      states = reg_states ++ [reset_state]
      time_locals = M.fromList
        [ (getIdString aid, 0)
        | aid <- concatMap (timeTaskDefs . sf_body . sched_fn) scheds ++
                 concatMap (timeTaskDefs . sf_body) (get_rule_fns top_block)
        ]
      state_widths = M.fromList [ (name, width) | (name, width, _) <- states ] `M.union` time_locals
      rule_fns = M.fromList [ (sf_name fn, fn) | fn <- get_rule_fns top_block ]
  default_actions <- m2StmtsToJson rule_fns state_widths reset (sf_body (sched_fn default_sched))
  generated_actions <- m2StmtsToJson rule_fns state_widths reset (sf_body (sched_fn generated_sched))
  ensure (not (resetTickAction reset_name `elem` default_actions))
         "M2 InitialReset tick appears on the default clock"
  ensure (case reverse generated_actions of
            action:_ -> action == resetTickAction reset_name
            [] -> False)
         "M2 InitialReset tick must be the final generated-clock action"
  return $ object
    [ field "schemaVersion" "2"
    , field "producer" $ object
        [ field "name" (jsonString "legacy-bsc-simcc-projection")
        , field "version" (jsonString "m2")
        ]
    , field "top" (jsonString top)
    , field "clocks" $ array
        [ m2ClockToJson default_clock 10 0 "low" 0 5 5
        , m2ClockToJson generated_clock (v1 + v2) 1
                        (if initial == 0 then "low" else "high") delay
                        (if initial == 0 then v1 else v2)
                        (if initial == 0 then v2 else v1)
        ]
    , field "state" $ array (map stateToJson states)
    , field "resets" $ array [m2ResetToJson reset cycles]
    , field "schedules" $ array
        [ m2ScheduleToJson default_clock default_actions
        , m2ScheduleToJson generated_clock generated_actions
        ]
    ]

m2PrimitiveInstances :: SimCCBlock -> Either String [(String, AId, [AExpr])]
m2PrimitiveInstances block = mapM primitive (sb_state block)
  where
    primitive (block_id, instance_id, args) =
      case primitiveName block_id of
        Just name -> return (name, instance_id, args)
        Nothing -> Left $ "M2 contains a non-primitive or unknown primitive block " ++ show block_id

m2RegState :: (AId, [AExpr]) -> Either String (String, Integer, Integer)
m2RegState (instance_id, args) =
  case args of
    [width_expr, initial_expr] -> do
      width <- m2Literal "RegN width" width_expr
      initial <- m2Literal "RegN initial value" initial_expr
      ensure (width > 0 && width <= 64) "M2 RegN width must be in 1..=64"
      ensure (initial < 2 ^ width) "M2 RegN initial value does not fit its width"
      return (getIdString instance_id, width, initial)
    _ -> Left $ "unsupported M2 RegN arguments: " ++ show args

m2ClockGenArgs :: [AExpr] -> Either String (Integer, Integer, Integer, Integer, Integer)
m2ClockGenArgs args =
  case args of
    [v1_expr, v2_expr, delay_expr, initial_expr, other_expr] -> do
      v1 <- m2Literal "ClockGen v1" v1_expr
      v2 <- m2Literal "ClockGen v2" v2_expr
      delay <- m2Literal "ClockGen delay" delay_expr
      initial <- m2Literal "ClockGen initial value" initial_expr
      other <- m2Literal "ClockGen other value" other_expr
      ensure (v1 > 0 && v2 > 0) "M2 ClockGen widths must be non-zero"
      ensure (v1 + v2 <= m2MaxValue) "M2 ClockGen period is out of range"
      ensure (initial == 0 || initial == 1) "M2 ClockGen initial value must be 0 or 1"
      ensure (other == 1 - initial) "M2 ClockGen other value must complement the initial value"
      return (v1, v2, delay, initial, other)
    _ -> Left $ "unsupported M2 ClockGen arguments: " ++ show args

m2InitialResetArgs :: [AExpr] -> Either String Integer
m2InitialResetArgs args = do
  cycles <- exactlyOne "InitialReset cycles" args >>= m2Literal "InitialReset cycles"
  ensure (cycles > 0) "M2 InitialReset cycles must be non-zero"
  return cycles

m2Literal :: String -> AExpr -> Either String Integer
m2Literal subject (ASInt _ (ATBit _) lit)
  | value >= 0 && value <= m2MaxValue = return value
  | otherwise = Left $ subject ++ " is out of range"
  where value = ilValue lit
m2Literal subject expr = Left $ subject ++ " must be an integer literal: " ++ show expr

m2MaxValue :: Integer
m2MaxValue = 2 ^ (64 :: Integer) - 1

m2Schedules :: String -> String -> [SimCCSched] -> Either String (SimCCSched, SimCCSched)
m2Schedules default_clock generated_clock scheds = do
  ensure (length scheds == 2) "M2 requires exactly two schedules"
  mapM_ m2PosedgeSchedule scheds
  default_sched <- exactlyOne "M2 default-clock schedule"
                   [ sched | sched <- scheds, clockName (sched_clock sched) == Right default_clock ]
  generated_sched <- exactlyOne "M2 ClockGen schedule"
                     [ sched | sched <- scheds, clockName (sched_clock sched) == Right generated_clock ]
  return (default_sched, generated_sched)

m2PosedgeSchedule :: SimCCSched -> Either String ()
m2PosedgeSchedule sched = do
  ensure (sched_posedge sched) "M2 supports only posedge schedules"
  ensure (sched_after_fn sched == Nothing) "M2 does not support after-edge schedules"

m2ResetDefinition :: SimCCBlock -> AId -> String -> [(String, Integer, Integer)]
                  -> Either String M2Reset
m2ResetDefinition block initial_reset clock reg_states = do
  ensure (null (sb_outputResets block)) "M2 does not support top-level output resets"
  default_reset <- exactlyOne "M2 default reset input" (sb_inputResets block)
  (signal, (source, output)) <- exactlyOne "M2 InitialReset source" (sb_resetSources block)
  ensure (sameId source initial_reset) "M2 reset source does not belong to InitialReset"
  ensure (getIdBaseString output == "gen_rst") "M2 InitialReset has an unexpected reset output"
  reset_type <- exactlyOne "M2 InitialReset signal definition"
                [ typ | (typ, reset_id) <- sb_resetDefs block, sameId reset_id signal ]
  case reset_type of
    ATBit 1 -> return ()
    _ -> Left "M2 InitialReset signal must have type Bit 1"
  reset_fn <- exactlyOne "M2 InitialReset reset function"
              [ fn | fn <- sb_resets block, sf_name fn == mkResetFnName signal ]
  target <- m2ResetFunctionTarget "InitialReset" signal reset_fn
  (_, _, target_value) <- exactlyOne "M2 InitialReset target RegN"
                         [ state | state@(name, _, _) <- reg_states, name == target ]
  other_reset_fn <- exactlyOne "M2 default reset function"
                    [ fn | fn <- sb_resets block, sf_name fn /= mkResetFnName signal ]
  default_target <- m2ResetFunctionTarget "default reset" default_reset other_reset_fn
  ensure (default_target /= target) "M2 default reset and InitialReset target the same RegN"
  ensure (any (\(name, _, _) -> name == default_target) reg_states)
         "M2 default reset must target a RegN"
  ensure (length (sb_resets block) == 2 && length (sb_resetDefs block) == 2 &&
          length (sb_resetSources block) == 1)
         "M2 supports only the default reset and one InitialReset"
  ensure (target_value >= 0) "M2 InitialReset target value is invalid"
  return (M2Reset (getIdString initial_reset) (getIdString signal) clock target
                  (getIdBaseString initial_reset) target_value)

m2ResetSignal :: M2Reset -> String
m2ResetSignal (M2Reset _ signal _ _ _ _) = signal

m2ResetFunctionTarget :: String -> AId -> SimCCFn -> Either String String
m2ResetFunctionTarget subject signal fn = do
  (_, argument) <- exactlyOne (subject ++ " reset function argument") (sf_args fn)
  ensure (sf_retType fn == Nothing) (subject ++ " reset function has a return value")
  case sf_body fn of
    [SFSAssign _ assigned value, SFSFunctionCall target method [reset_value]] -> do
      ensure (sameId assigned signal) (subject ++ " reset function writes the wrong signal")
      ensure (isResetArgument argument value && isResetArgument argument reset_value)
             (subject ++ " reset function has an unexpected reset argument")
      ensure (method == "reset_RST")
             (subject ++ " reset function calls an unexpected method")
      return (getIdBaseString target)
    _ -> Left $ "unsupported " ++ subject ++ " reset function: " ++ show fn

isResetArgument :: AId -> AExpr -> Bool
isResetArgument argument (ASDef _ value) = sameId argument value
isResetArgument argument (ASPort _ value) = sameId argument value
isResetArgument _ _ = False

sameId :: AId -> AId -> Bool
sameId left right = getIdString left == getIdString right

m2ClockToJson :: String -> Integer -> Integer -> String -> Integer -> Integer -> Integer -> String
m2ClockToJson name period order initial_value first_edge high_duration low_duration = object
  [ field "id" (jsonString name)
  , field "period" (show period)
  , field "order" (show order)
  , field "initialValue" (jsonString initial_value)
  , field "firstEdge" (show first_edge)
  , field "highDuration" (show high_duration)
  , field "lowDuration" (show low_duration)
  , field "activeEdge" (jsonString "posedge")
  ]

m2ResetToJson :: M2Reset -> Integer -> String
m2ResetToJson (M2Reset reset_id signal clock target _ value) cycles = object
  [ field "id" (jsonString reset_id)
  , field "signal" (jsonString signal)
  , field "clock" (jsonString clock)
  , field "cycles" (show cycles)
  , field "targets" $ array
      [ object
        [ field "state" (jsonString target)
        , field "value" (show value)
        ]
      ]
  ]

m2ScheduleToJson :: String -> [String] -> String
m2ScheduleToJson clock actions = object
  [ field "clock" (jsonString clock)
  , field "actions" (array actions)
  ]

resetTickAction :: String -> String
resetTickAction reset_id = object
  [ field "kind" (jsonString "reset_tick")
  , field "reset" (jsonString reset_id)
  ]

ensure :: Bool -> String -> Either String ()
ensure True _ = Right ()
ensure False message = Left message

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

-- M2 differs only in the two primitive calls synthesized at schedule level:
-- InitialReset.clk is represented by reset_tick, while the unrelated default
-- reset tick remains outside the exported model.
m2StmtsToJson :: M.Map String SimCCFn -> M.Map String Integer -> M2Reset -> [SimCCFnStmt]
              -> Either String [String]
m2StmtsToJson rule_fns state_widths reset =
  fmap concat . mapM (m2StmtToJson rule_fns state_widths reset)

m2StmtToJson :: M.Map String SimCCFn -> M.Map String Integer -> M2Reset -> SimCCFnStmt
             -> Either String [String]
m2StmtToJson rule_fns state_widths (M2Reset reset_id _ _ reset_target reset_instance _) stmt =
  case stmt of
    SFSFunctionCall object name args
      | getIdBaseString object == reset_instance && name == "clk" -> do
          ensure (all isOneBit args && length args == 2)
                 "M2 InitialReset clock call has unexpected arguments"
          return []
      | otherwise -> stmtToJson rule_fns state_widths stmt
    SFSResets [SFSFunctionCall object name [arg]]
      | "rst_tick__clk__" `isInfixOf` name && isOneBit arg ->
          if getIdBaseString object == reset_target
            then return [resetTickAction reset_id]
            else return []
    _ -> stmtToJson rule_fns state_widths stmt
isOneBit :: AExpr -> Bool
isOneBit (ASInt _ (ATBit 1) lit) = ilValue lit == 1
isOneBit _ = False

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
    APrim { ae_type = ATBit width, aprim_prim = PrimULE, ae_args = [left, right] }
      | width == 1 -> do
          left_json <- exprToJson state_widths left
          right_json <- exprToJson state_widths right
          return $ object
            [ field "kind" (jsonString "unary")
            , field "width" "1"
            , field "op" (jsonString "not")
            , field "arg" $ object
                [ field "kind" (jsonString "binary")
                , field "width" "1"
                , field "op" (jsonString "unsigned_less_than")
                , field "args" (array [right_json, left_json])
                ]
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
