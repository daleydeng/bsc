module AExpr2Z3(
       ZState,
       initZState,
       addADefToZState,

       checkDisjointExpr,
       checkDisjointRulePair,

       checkBiImplication,
       isConstExpr,
       checkEq,
       checkNotEq
) where

import Control.Monad(when)
import Control.Monad.State(StateT, runStateT, liftIO, gets, get, put, modify)
import qualified Data.Map as M

import ErrorUtil(internalError)
import Flags
import Id(dummy_id)
import Prim
import IntLit
import ASyntax
import ASyntaxUtil(aAnds, aSize)
import VModInfo(VModInfo)
import PFPrint
import Util(itos, map_insertMany, makePairs)
import AExpr2Util(getSingleMethodOutputPort, getMethodOutputPortAt)
import qualified SMTLib2Z3 as Z3

import Debug.Trace(traceM)
import IOUtil(progArgs)

traceTest :: Bool
traceTest = "-trace-smt-test" `elem` progArgs

traceConv :: Bool
traceConv = "-trace-smt-conv" `elem` progArgs

-- -------------------------

type RuleTriple = (ARuleId, [AExpr], Maybe ARuleId)

data ZType = ZBits Integer | ZBool | ZZero
             deriving (Eq, Ord, Show)

type ZExpr = (String, ZType)

data ZState =
    ZState {
               hardFail      :: Bool,
               defMap        :: M.Map AId ADef,
               ruleMap       :: M.Map ARuleId RuleTriple,
               stateMap      :: M.Map AId VModInfo,
               proofMap      :: M.Map (AExpr, AExpr) Z3.SMTResult,

               anyId         :: Integer,
               unknownId     :: Integer,
               portId        :: Integer,

               defZExprMap   :: M.Map AId ZExpr,
               portZExprMap  :: M.Map AId ZExpr,
               expZExprMap   :: M.Map AExpr ZExpr,
               declarations  :: M.Map String ZType
              }

type ZM = StateT ZState IO

initZState :: String -> Flags -> Bool ->
              [ADef] -> [AVInst] -> [RuleTriple] -> IO ZState
initZState _ _ doHardFail ds avis rs = do
  ver <- Z3.checkVersion
  when traceTest $ putStrLn $ "Using " ++ ver
  return (ZState { hardFail = doHardFail,
                   defMap = M.fromList [(i,d) | d@(ADef i _ _ _) <- ds],
                   ruleMap = M.fromList [(i,r) | r@(i,_,_) <- rs],
                   stateMap = M.fromList [(avi_vname avi, avi_vmi avi)
                                            | avi <- avis],
                   proofMap = M.empty,
                   anyId = 0,
                   unknownId = 0,
                   portId = 0,
                   defZExprMap = M.empty,
                   portZExprMap = M.empty,
                   expZExprMap = M.empty,
                   declarations = M.empty
                 })

addADefToZState :: ZState -> [ADef] -> IO ZState
addADefToZState s ds = do
  let dmap' = map_insertMany [(i, d) | d@(ADef i _ _ _) <- ds] (defMap s)
  return (s { defMap = dmap' })

-- -------------------------
-- Queries

checkDisjointExpr :: ZState -> AExpr -> AExpr -> IO (Maybe Bool, ZState)
checkDisjointExpr s e1 e2 = runStateT (checkDisjointExprM e1 e2) s

checkDisjointExprM :: AExpr -> AExpr -> ZM (Maybe Bool)
checkDisjointExprM e1 e2 = do
  when traceTest $ traceM("Z3 comparing exprs: " ++ ppString (e1, e2))
  cached <- gets (M.lookup (e1,e2) . proofMap)
  satRes <- case cached of
    Just r -> return r
    Nothing -> do
      (z1, _) <- forceBool e1
      (z2, _) <- forceBool e2
      r <- runQuery (app "and" [z1,z2])
      modify $ \s -> s { proofMap = M.insert (e2,e1) r
                                      (M.insert (e1,e2) r (proofMap s)) }
      return r
  return $ case satRes of
    Z3.SMTUnsat -> Just True
    Z3.SMTSat -> Just False
    Z3.SMTUnknown -> Nothing

checkDisjointRulePair :: ZState -> (ARuleId, ARuleId) ->
                         IO (Maybe Bool, ZState)
checkDisjointRulePair s pair = runStateT (checkDisjointRulePairM pair) s

checkDisjointRulePairM :: (ARuleId, ARuleId) -> ZM (Maybe Bool)
checkDisjointRulePairM (r1,r2) = do
  c1 <- getRuleCond r1
  c2 <- getRuleCond r2
  checkDisjointExprM c1 c2

getRuleCond :: ARuleId -> ZM AExpr
getRuleCond rid = do
  rmap <- gets ruleMap
  case M.lookup rid rmap of
    Just (_, conds, _) -> return (aAnds conds)
    Nothing -> internalError ("AExpr2Z3.getRuleCond: cannot find rule: " ++
                              ppReadable rid)

checkBiImplication :: ZState -> AExpr -> AExpr ->
                      IO ((Bool, Bool), ZState)
checkBiImplication s e1 e2 = runStateT action s
  where action = do
          (z1, _) <- forceBool e1
          (z2, _) <- forceBool e2
          r12 <- runQuery (app "and" [z1, app "not" [z2]])
          r21 <- runQuery (app "and" [z2, app "not" [z1]])
          return (r12 == Z3.SMTUnsat, r21 == Z3.SMTUnsat)

isConstExpr :: ZState -> AExpr -> IO (Maybe Bool, ZState)
isConstExpr s e = runStateT action s
  where action = do
          (ze, _) <- forceBool e
          notSat <- runQuery (app "not" [ze])
          if notSat == Z3.SMTUnsat
            then return (Just True)
            else do sat <- runQuery ze
                    return $ if sat == Z3.SMTUnsat
                             then Just False
                             else Nothing

checkEq :: ZState -> AExpr -> AExpr -> IO (Maybe Bool, ZState)
checkEq s e1 e2 = runStateT (checkRelation True e1 e2) s

checkNotEq :: ZState -> AExpr -> AExpr -> IO (Maybe Bool, ZState)
checkNotEq s e1 e2 = runStateT (checkRelation False e1 e2) s

checkRelation :: Bool -> AExpr -> AExpr -> ZM (Maybe Bool)
checkRelation provingEq e1 e2 = do
  z1 <- convAExpr Nothing e1
  z2raw <- convAExpr (Just (snd z1)) e2
  z2 <- convType (Just (snd z1)) z2raw
  eq <- eqExpr z1 z2
  let query = if provingEq then app "not" [eq] else eq
  res <- runQuery query
  return $ case res of
    Z3.SMTUnsat -> Just True
    Z3.SMTSat -> Just False
    Z3.SMTUnknown -> Nothing

runQuery :: String -> ZM Z3.SMTResult
runQuery assertion = do
  decls <- gets declarations
  let commands = map renderDeclaration (M.toAscList decls)
  when traceTest $ traceM ("Z3 assertion: " ++ assertion)
  liftIO $ Z3.runZ3 commands assertion

renderDeclaration :: (String, ZType) -> String
renderDeclaration (name, ZBool) = "(declare-const " ++ name ++ " Bool)"
renderDeclaration (name, ZBits w) =
  "(declare-const " ++ name ++ " (_ BitVec " ++ show w ++ "))"
renderDeclaration (_, ZZero) = internalError "AExpr2Z3: zero-width declaration"

-- -------------------------
-- State helpers

addToDefMap :: AId -> ZExpr -> ZM ()
addToDefMap i z = modify $ \s -> s { defZExprMap = M.insert i z (defZExprMap s) }

addToPortMap :: AId -> ZExpr -> ZM ()
addToPortMap i z = modify $ \s -> s { portZExprMap = M.insert i z (portZExprMap s) }

addToExpMap :: AExpr -> ZExpr -> ZM ()
addToExpMap e z = modify $ \s -> s { expZExprMap = M.insert e z (expZExprMap s) }

freshName :: String -> (ZState -> Integer) ->
             (ZState -> Integer -> ZState) -> ZM String
freshName prefix getter setter = do
  s <- get
  let n = getter s
  put (setter s (n + 1))
  return (prefix ++ itos n)

freshAnyName :: ZM String
freshAnyName = freshName "__bsc_any_" anyId (\s n -> s { anyId = n })

freshUnknownName :: ZM String
freshUnknownName = freshName "__bsc_unknown_" unknownId
                                 (\s n -> s { unknownId = n })

freshPortName :: ZM String
freshPortName = freshName "__bsc_port_" portId (\s n -> s { portId = n })

makeVar :: Maybe ZType -> String -> Integer -> ZM ZExpr
makeVar _ _ 0 = return ("__bsc_zero_width", ZZero)
makeVar (Just ZBool) name width
  | width == 1 = declare name ZBool
  | otherwise = internalError ("AExpr2Z3.makeVar: Bool width " ++ show width)
makeVar _ name width = declare name (ZBits width)

declare :: String -> ZType -> ZM ZExpr
declare name ty = do
  modify $ \s -> s { declarations = M.insert name ty (declarations s) }
  return (name, ty)

addUnknownExpr :: Maybe ZType -> AExpr -> Integer -> ZM ZExpr
addUnknownExpr mty e width = do
  emap <- gets expZExprMap
  case M.lookup e emap of
    Just z -> convType mty z
    Nothing -> do
      name <- freshUnknownName
      z <- makeVar mty name width
      addToExpMap e z
      return z

-- -------------------------
-- Bool / bit-vector conversion

convType :: Maybe ZType -> ZExpr -> ZM ZExpr
convType Nothing z = return z
convType (Just ZBool) z@(_, ZBool) = return z
convType (Just (ZBits _)) z@(_, ZBits _) = return z
convType (Just ZZero) z@(_, ZZero) = return z
convType (Just ZBool) (term, ZBits 1) =
  return (app "=" [term, bvConst 1 1], ZBool)
convType (Just (ZBits 1)) (term, ZBool) =
  return (app "ite" [term, bvConst 1 1, bvConst 1 0], ZBits 1)
convType (Just expected) (_, actual) =
  internalError ("AExpr2Z3.convType: cannot convert " ++ show actual ++
                 " to " ++ show expected)

forceBool :: AExpr -> ZM ZExpr
forceBool e = convAExpr (Just ZBool) e >>= convType (Just ZBool)

forceBits :: AExpr -> ZM ZExpr
forceBits e =
  let ty = case ae_type e of
             ATBit 0 -> ZZero
             ATBit w -> ZBits w
             t -> internalError ("AExpr2Z3.forceBits: " ++ ppReadable t)
  in convAExpr (Just ty) e >>= convType (Just ty)

getBitType :: AExpr -> ZType
getBitType e = case ae_type e of
  ATBit 0 -> ZZero
  ATBit w -> ZBits w
  t -> internalError ("AExpr2Z3.getBitType: " ++ ppReadable t)

-- -------------------------
-- AExpr conversion

convAExpr :: Maybe ZType -> AExpr -> ZM ZExpr
convAExpr (Just ZBool) (ASInt _ (ATBit width) (IntLit _ _ value))
  | width /= 1 = internalError ("AExpr2Z3: invalid Bool width " ++ show width)
  | value == 0 = return ("false", ZBool)
  | value == 1 = return ("true", ZBool)
  | otherwise = internalError ("AExpr2Z3: invalid Bool value " ++ show value)
convAExpr _ (ASInt _ (ATBit 0) _) = return ("__bsc_zero_width", ZZero)
convAExpr _ (ASInt _ (ATBit width) (IntLit _ _ value)) =
  return (bvConst width value, ZBits width)

convAExpr mty e@(ASDef (ATBit width) aid) = do
  zmap <- gets defZExprMap
  case M.lookup aid zmap of
    Just z -> convType mty z
    Nothing -> do
      dmap <- gets defMap
      case M.lookup aid dmap of
        Just ADef { adef_expr = e' } -> do
          z <- convAExpr mty e'
          addToDefMap aid z
          return z
        Nothing -> do
          mustFail <- gets hardFail
          if mustFail
            then internalError ("AExpr2Z3: missing def: " ++ ppReadable aid)
            else addUnknownExpr mty e width

convAExpr mty (ASPort (ATBit width) aid) = do
  zmap <- gets portZExprMap
  case M.lookup aid zmap of
    Just z -> convType mty z
    Nothing -> do
      name <- freshPortName
      z <- makeVar mty name width
      addToPortMap aid z
      return z

convAExpr mty (ASParam ty@(ATBit _) aid) = convAExpr mty (ASPort ty aid)

-- ASAny occurrences are intentionally independent, even when structurally equal.
convAExpr mty (ASAny (ATBit width) _) = do
  name <- freshAnyName
  makeVar mty name width

convAExpr mty (APrim i (ATBit width) prim args) =
  convPrim mty prim i width args

convAExpr mty (AMethCall ty@(ATBit width) modId methId args) = do
  smap <- gets stateMap
  let e = AMethCall ty modId (getSingleMethodOutputPort smap modId methId) args
  addUnknownExpr mty e width
convAExpr mty (AMethValue ty@(ATBit width) modId methId) = do
  smap <- gets stateMap
  let e = AMethValue ty modId (getSingleMethodOutputPort smap modId methId)
  addUnknownExpr mty e width
convAExpr mty (ATupleSel ty@(ATBit width)
                 (AMethCall _ modId methId args) selIdx) = do
  smap <- gets stateMap
  let e = AMethCall ty modId (getMethodOutputPortAt smap modId methId selIdx) args
  addUnknownExpr mty e width
convAExpr mty (ATupleSel ty@(ATBit width)
                 (AMethValue _ modId methId) selIdx) = do
  smap <- gets stateMap
  let e = AMethValue ty modId (getMethodOutputPortAt smap modId methId selIdx)
  addUnknownExpr mty e width

convAExpr mty e@(AMGate (ATBit 1) _ _) = addUnknownExpr mty e 1
convAExpr mty e@(ASStr _ (ATString (Just _)) _) = addUnknownExpr mty e (aSize e)
convAExpr mty e@(ANoInlineFunCall (ATBit width) _ _ _) = addUnknownExpr mty e width
convAExpr mty e@(AFunCall (ATBit width) _ _ _ _) = addUnknownExpr mty e width
convAExpr mty e@(ATaskValue (ATBit width) _ _ _ _) = addUnknownExpr mty e width
convAExpr _ e = internalError ("unexpected expr/type in AExpr2Z3: " ++ show e)

-- -------------------------
-- Primitive conversion

convPrim :: Maybe ZType -> PrimOp -> AId -> Integer -> [AExpr] -> ZM ZExpr
convPrim mty PrimIf _ _ [c,t,f] = do
  (zc, _) <- forceBool c
  zt0 <- convAExpr mty t
  zf0 <- convAExpr mty f
  (zt,zf,ty) <- if snd zt0 == snd zf0
                then return (fst zt0, fst zf0, snd zt0)
                else do zt1 <- convType mty zt0
                        zf1 <- convType (Just (snd zt1)) zf0
                        return (fst zt1, fst zf1, snd zf1)
  case ty of
    ZZero -> return ("__bsc_zero_width", ZZero)
    _ -> return (app "ite" [zc,zt,zf], ty)

convPrim mty PrimCase i w (idx:dflt:cases) =
  let foldFn (v,e) res =
        let c = APrim i aTBool PrimEQ [idx,v]
        in APrim i (ATBit w) PrimIf [c,e,res]
  in convAExpr mty (foldr foldFn dflt (makePairs cases))

convPrim mty PrimArrayDynSelect i w args =
  case args of
    [APrim _ _ PrimBuildArray es, idx] ->
      let idxTy = ae_type idx
          maxIdx = case idxTy of
                     ATBit sz -> (2 ^ sz) - 1
                     _ -> internalError "AExpr2Z3: array index is not bits"
          arms = zip [0..maxIdx] es
          dflt = ASAny (ATBit w) Nothing
          foldFn (n,e) res =
            let lit = ASInt i idxTy (ilDec n)
                c = APrim i aTBool PrimEQ [idx,lit]
            in APrim i (ATBit w) PrimIf [c,e,res]
      in convAExpr mty (foldr foldFn dflt arms)
    [ASDef _ defId, idx] -> do
      dmap <- gets defMap
      case M.lookup defId dmap of
        Just ADef { adef_expr = e } -> convPrim mty PrimArrayDynSelect i w [e,idx]
        _ -> internalError ("AExpr2Z3 PrimArrayDynSelect: " ++ ppReadable args)
    _ -> internalError ("AExpr2Z3 PrimArrayDynSelect: " ++ ppReadable args)

convPrim _ PrimEQ _ _ [a,b] = do
  za <- convAExpr Nothing a
  zb0 <- convAExpr (Just (snd za)) b
  zb <- convType (Just (snd za)) zb0
  z <- eqExpr za zb
  return (z,ZBool)
convPrim _ PrimEQ _ _ args = wrongArgs "PrimEQ" args

convPrim _ PrimBOr _ _ args = boolMany "or" args
convPrim _ PrimBAnd _ _ args = boolMany "and" args
convPrim _ PrimBNot _ _ [a] = do
  (za,_) <- forceBool a
  return (app "not" [za],ZBool)
convPrim _ PrimBNot _ _ args = wrongArgs "PrimBNot" args

convPrim _ PrimULE _ _ args = bitsCompare "bvule" args
convPrim _ PrimULT _ _ args = bitsCompare "bvult" args
convPrim _ PrimSLE _ _ args = bitsCompare "bvsle" args
convPrim _ PrimSLT _ _ args = bitsCompare "bvslt" args

convPrim _ PrimAnd _ w args = bitsBinary w "bvand" args
convPrim _ PrimOr  _ w args = bitsBinary w "bvor" args
convPrim _ PrimXor _ w args = bitsBinary w "bvxor" args
convPrim _ PrimInv _ w [a] = bitsUnary w "bvnot" a
convPrim _ PrimInv _ _ args = wrongArgs "PrimInv" args

convPrim _ PrimAdd _ w args = bitsBinary w "bvadd" args
convPrim _ PrimSub _ w args = bitsBinary w "bvsub" args
convPrim _ PrimMul _ w args =
  bitsBinary w "bvmul" (map (aZeroExtend w) args)
convPrim _ PrimQuot _ w args = wideBinaryThenTruncate w "bvudiv" args
convPrim _ PrimRem _ w args = wideBinaryThenTruncate w "bvurem" args
convPrim _ PrimNeg _ w [a] = bitsUnary w "bvneg" a
convPrim _ PrimNeg _ _ args = wrongArgs "PrimNeg" args

convPrim _ PrimSL _ w args = wideBinaryThenTruncate w "bvshl" args
convPrim _ PrimSRL _ w args = wideBinaryThenTruncate w "bvlshr" args
convPrim _ PrimSRA _ w [a,b] =
  let wide = maximum (map (getWidth . ae_type) [a,b])
  in bitsBinary wide "bvashr" [aSignExtend wide a, aZeroExtend wide b]
       >>= truncateExpr w
convPrim _ PrimSRA _ _ args = wrongArgs "PrimSRA" args

convPrim (Just ZBool) PrimExtract _ _
         [e, ASInt _ _ (IntLit _ _ hi), ASInt _ _ (IntLit _ _ lo)]
  | hi == lo = do
      (ze,_) <- forceBits e
      return (app ("(_ extract " ++ show hi ++ " " ++ show lo ++ ")") [ze]
             `eqTerm` bvConst 1 1, ZBool)
convPrim _ PrimExtract _ _
         [e, ASInt _ _ (IntLit _ _ hi), ASInt _ _ (IntLit _ _ lo)] = do
  (ze,_) <- forceBits e
  let width = hi - lo + 1
  return (app ("(_ extract " ++ show hi ++ " " ++ show lo ++ ")") [ze],
          if width == 0 then ZZero else ZBits width)
convPrim mty PrimExtract i w args =
  addUnknownExpr mty (APrim i (ATBit w) PrimExtract args) w

convPrim _ PrimConcat _ w args = concatArgs w args

convPrim _ PrimSignExt _ w [e] = do
  z@(_,ty) <- forceBits e
  case ty of
    ZZero -> return (bvConst w 0, if w == 0 then ZZero else ZBits w)
    ZBits ew
      | ew == w -> return z
      | ew < w -> return (app ("(_ sign_extend " ++ show (w-ew) ++ ")") [fst z], ZBits w)
      | otherwise -> internalError "AExpr2Z3: negative sign extension"
    _ -> internalError "AExpr2Z3: sign extension of Bool"
convPrim _ PrimSignExt _ _ args = wrongArgs "PrimSignExt" args

-- These commonly survive simplification and have direct SMT-LIB encodings.
convPrim _ PrimZeroExt _ w [e] = do
  z@(_,ty) <- forceBits e
  case ty of
    ZZero -> return (bvConst w 0, if w == 0 then ZZero else ZBits w)
    ZBits ew
      | ew == w -> return z
      | ew < w -> return (app ("(_ zero_extend " ++ show (w-ew) ++ ")") [fst z], ZBits w)
      | otherwise -> internalError "AExpr2Z3: negative zero extension"
    _ -> internalError "AExpr2Z3: zero extension of Bool"
convPrim _ PrimTrunc _ w [e] = forceBits e >>= truncateExpr w

convPrim mty prim i w args =
  addUnknownExpr mty (APrim i (ATBit w) prim args) w

-- -------------------------
-- Primitive helpers

wrongArgs :: String -> [AExpr] -> ZM a
wrongArgs name args = internalError ("AExpr2Z3." ++ name ++
                                     ": wrong number of args: " ++
                                     show (length args))

boolMany :: String -> [AExpr] -> ZM ZExpr
boolMany op args
  | length args < 2 = wrongArgs op args
  | otherwise = do zs <- mapM forceBool args
                   return (app op (map fst zs), ZBool)

bitsCompare :: String -> [AExpr] -> ZM ZExpr
bitsCompare op [a,b] = do
  za <- forceBits a
  zb <- forceBits b
  case (snd za, snd zb) of
    (ZZero,ZZero) ->
      return (if op `elem` ["bvule","bvsle"] then "true" else "false", ZBool)
    (ZBits wa,ZBits wb)
      | wa == wb -> return (app op [fst za,fst zb],ZBool)
      | otherwise -> internalError ("AExpr2Z3: comparison width mismatch " ++ show (wa,wb))
    _ -> internalError "AExpr2Z3: comparison type mismatch"
bitsCompare op args = wrongArgs op args

bitsBinary :: Integer -> String -> [AExpr] -> ZM ZExpr
bitsBinary w op [a,b]
  | w == 0 = return ("__bsc_zero_width",ZZero)
  | otherwise = do
      za <- forceBits a
      zb <- forceBits b
      case (snd za,snd zb) of
        (ZBits wa,ZBits wb)
          | wa == wb && wa == w -> return (app op [fst za,fst zb],ZBits w)
          | otherwise -> internalError ("AExpr2Z3: binary width mismatch " ++ show (w,wa,wb))
        _ -> internalError "AExpr2Z3: binary operation on zero-width value"
bitsBinary _ op args = wrongArgs op args

bitsUnary :: Integer -> String -> AExpr -> ZM ZExpr
bitsUnary 0 _ _ = return ("__bsc_zero_width",ZZero)
bitsUnary w op a = do
  (za,ty) <- forceBits a
  case ty of
    ZBits wa | wa == w -> return (app op [za],ZBits w)
    _ -> internalError ("AExpr2Z3: unary width mismatch " ++ show (w,ty))

wideBinaryThenTruncate :: Integer -> String -> [AExpr] -> ZM ZExpr
wideBinaryThenTruncate w op args =
  let wide = maximum (map (getWidth . ae_type) args)
      args' = map (aZeroExtend wide) args
  in bitsBinary wide op args' >>= truncateExpr w

concatArgs :: Integer -> [AExpr] -> ZM ZExpr
concatArgs w args = do
  zs <- mapM forceBits args
  let nonzero = [(z,n) | (z,ZBits n) <- zs]
  case nonzero of
    [] -> return ("__bsc_zero_width",ZZero)
    [(z,n)] | n == w -> return (z,ZBits n)
    _ -> let actual = sum (map snd nonzero)
         in if actual /= w
            then internalError ("AExpr2Z3: concat width mismatch " ++ show (w,actual))
            else return (foldl1 (\a b -> app "concat" [a,b]) (map fst nonzero), ZBits w)

truncateExpr :: Integer -> ZExpr -> ZM ZExpr
truncateExpr w z@(_,ZBits ew)
  | w == ew = return z
  | w == 0 = return ("__bsc_zero_width",ZZero)
  | w < ew = return (app ("(_ extract " ++ show (w-1) ++ " 0)") [fst z], ZBits w)
  | otherwise = internalError ("AExpr2Z3: cannot truncate " ++ show ew ++ " to " ++ show w)
truncateExpr 0 (_,ZZero) = return ("__bsc_zero_width",ZZero)
truncateExpr w (_,ty) = internalError ("AExpr2Z3: cannot truncate " ++ show ty ++ " to " ++ show w)

eqExpr :: ZExpr -> ZExpr -> ZM String
eqExpr (_,ZZero) (_,ZZero) = return "true"
eqExpr (a,ta) (b,tb)
  | ta == tb = return (app "=" [a,b])
  | otherwise = internalError ("AExpr2Z3.eqExpr: type mismatch " ++ show (ta,tb))

-- -------------------------
-- AST width helpers

getWidth :: AType -> Integer
getWidth (ATBit w) = w
getWidth t = internalError ("AExpr2Z3.getWidth: " ++ ppReadable t)

aZeroExtend :: Integer -> AExpr -> AExpr
aZeroExtend w e =
  let ew = getWidth (ae_type e)
  in case compare w ew of
       EQ -> e
       GT -> APrim dummy_id (ATBit w) PrimConcat
               [ASInt defaultAId (ATBit (w-ew)) (ilDec 0),e]
       LT -> internalError ("AExpr2Z3.aZeroExtend: " ++ ppReadable (w,e))

aSignExtend :: Integer -> AExpr -> AExpr
aSignExtend w e =
  let ew = getWidth (ae_type e)
  in case compare w ew of
       EQ -> e
       GT -> APrim dummy_id (ATBit w) PrimSignExt [e]
       LT -> internalError ("AExpr2Z3.aSignExtend: " ++ ppReadable (w,e))

-- -------------------------
-- SMT-LIB rendering

app :: String -> [String] -> String
app op args = "(" ++ unwords (op:args) ++ ")"

infix 4 `eqTerm`
eqTerm :: String -> String -> String
eqTerm a b = app "=" [a,b]

bvConst :: Integer -> Integer -> String
bvConst width value
  | width <= 0 = internalError ("AExpr2Z3.bvConst: invalid width " ++ show width)
  | otherwise = "(_ bv" ++ show (value `mod` (2 ^ width)) ++ " " ++ show width ++ ")"
