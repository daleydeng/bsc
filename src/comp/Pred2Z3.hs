module Pred2Z3(
       ZState,
       initZState,
       solvePred
) where

import Control.Monad.State(StateT, runStateT, liftIO, gets, get, put, modify)
import qualified Data.Map as M

import Id
import PreIds
import CType
import Type
import Pred
import qualified SMTLib2Z3 as Z3

import Debug.Trace(traceM)
import IOUtil(progArgs)
import PFPrint(ppReadable)

traceTest :: Bool
traceTest = "-trace-smt-test" `elem` progArgs

traceConv :: Bool
traceConv = "-trace-smt-conv" `elem` progArgs

-- Numeric types are non-negative mathematical integers.  This avoids the
-- overflow-based false proofs possible with the historical 32-bit encoding.
data ZState = ZState {
    nextId       :: Integer,
    typeExprMap  :: M.Map Type (String, [String]),
    declarations :: M.Map String ()
  }

type ZM = StateT ZState IO

initZState :: IO ZState
initZState = do
  ver <- Z3.checkVersion
  if traceTest then putStrLn ("Using " ++ ver ++ " for numeric provisos") else return ()
  return ZState { nextId = 0, typeExprMap = M.empty, declarations = M.empty }

solvePred :: ZState -> [Pred] -> Pred -> IO (Maybe Pred, ZState)
solvePred s ps p = runStateT (solvePredM ps p) s

solvePredM :: [Pred] -> Pred -> ZM (Maybe Pred)
solvePredM ps p = do
  if traceTest then traceM ("Z3 solvePred: " ++ ppReadable p) else return ()
  mneq <- predInequality p
  case mneq of
    Nothing -> return Nothing
    Just (targetIneq, targetConstraints) -> do
      -- Do not prove anything from an inconsistent set of provisos.
      consistency <- predicatesAssertions (p:ps)
      sat <- runQuery consistency
      if sat /= Z3.SMTSat
        then return Nothing
        else do
          assumptions <- predicatesAssertions ps
          result <- runQuery (targetIneq:targetConstraints ++ assumptions)
          return $ if result == Z3.SMTUnsat then Just p else Nothing

runQuery :: [String] -> ZM Z3.SMTResult
runQuery assertions = do
  decls <- gets declarations
  let commands =
        ["(define-fun-rec __bsc_pow2 ((n Int)) Int " ++
         "(ite (<= n 0) 1 (* 2 (__bsc_pow2 (- n 1)))))",
         "(define-fun-rec __bsc_clog2 ((n Int)) Int " ++
         "(ite (<= n 1) 0 (+ 1 (__bsc_clog2 (div (+ n 1) 2)))))"] ++
        ["(declare-const " ++ name ++ " Int)" | name <- M.keys decls]
  liftIO $ Z3.runZ3 commands (andAll assertions)

-- -------------------------
-- Predicates

predInequality :: Pred -> ZM (Maybe (String,[String]))
predInequality p = do
  parts <- predEqualityParts p
  return $ case parts of
    Nothing -> Nothing
    Just (eq,constraints) -> Just (app "not" [eq],constraints)

predicatesAssertions :: [Pred] -> ZM [String]
predicatesAssertions ps = do
  ass <- mapM predEqualityWithConstraints ps
  return (concat ass)

predEqualityWithConstraints :: Pred -> ZM [String]
predEqualityWithConstraints p = do
  meq <- predEqualityParts p
  case meq of
    Nothing -> return []
    Just (eq, constraints) -> return (eq:constraints)



predEqualityParts :: Pred -> ZM (Maybe (String, [String]))
predEqualityParts (IsIn c [t1,t2]) | classId c == idNumEq =
  equalityOf t1 t2
predEqualityParts (IsIn c [t1,t2,t3]) | classId c == idAdd =
  equalityOf (TAp (TAp tAdd t1) t2) t3
predEqualityParts (IsIn c [t1,t2,t3]) | classId c == idMul =
  equalityOf (TAp (TAp tMul t1) t2) t3
predEqualityParts (IsIn c [t1,t2,t3]) | classId c == idMax =
  equalityOf (TAp (TAp tMax t1) t2) t3
predEqualityParts (IsIn c [t1,t2,t3]) | classId c == idMin =
  equalityOf (TAp (TAp tMin t1) t2) t3
predEqualityParts (IsIn c [t1,t2,t3]) | classId c == idDiv =
  equalityOf (TAp (TAp tDiv t1) t2) t3
predEqualityParts (IsIn c [t1,t2]) | classId c == idLog =
  equalityOf (TAp tLog t1) t2
predEqualityParts _ = return Nothing

equalityOf :: Type -> Type -> ZM (Maybe (String,[String]))
equalityOf t1 t2 = do
  (z1,c1) <- convType t1
  (z2,c2) <- convType t2
  return (Just (app "=" [z1,z2], c1 ++ c2))

classId :: Class -> Id
classId = typeclassId . name

-- -------------------------
-- Type expressions

convType :: Type -> ZM (String,[String])
convType t = do
  cache <- gets typeExprMap
  case M.lookup t cache of
    Just z -> return z
    Nothing -> do
      if traceConv then traceM ("Z3 converting type: " ++ ppReadable t) else return ()
      z <- convType' t
      modify $ \s -> s { typeExprMap = M.insert t z (typeExprMap s) }
      return z

convType' :: Type -> ZM (String,[String])
convType' (TCon (TyNum n _)) = return (show n,[])
convType' t@(TVar _) = unknownType t
convType' t@(TAp (TAp tc t1) t2) | tc == tAdd = binaryType "+" t1 t2
convType' t@(TAp (TAp tc t1) t2) | tc == tSub = do
  (z1,c1) <- convType t1
  (z2,c2) <- convType t2
  return (app "-" [z1,z2], app ">=" [z1,z2] : c1 ++ c2)
convType' t@(TAp (TAp tc t1) t2) | tc == tMul = binaryType "*" t1 t2
convType' t@(TAp (TAp tc t1) t2) | tc == tMax = minMaxType ">=" t1 t2
convType' t@(TAp (TAp tc t1) t2) | tc == tMin = minMaxType "<=" t1 t2
convType' t@(TAp (TAp tc t1) t2) | tc == tDiv = do
  (z1,c1) <- convType t1
  (z2,c2) <- convType t2
  -- Bluespec TDiv is ceiling division for non-negative numeric types.
  let result = app "div" [app "+" [z1,app "-" [z2,"1"]],z2]
  return (result, app ">" [z2,"0"] : c1 ++ c2)
convType' (TAp tc arg) | tc == tExp = do
  (z,constraints) <- convType arg
  return (app "__bsc_pow2" [z],constraints)
convType' (TAp tc arg) | tc == tLog = do
  (z,constraints) <- convType arg
  return (app "__bsc_clog2" [z],app ">" [z,"0"] : constraints)
convType' t = unknownType t

binaryType :: String -> Type -> Type -> ZM (String,[String])
binaryType op t1 t2 = do
  (z1,c1) <- convType t1
  (z2,c2) <- convType t2
  return (app op [z1,z2],c1 ++ c2)

minMaxType :: String -> Type -> Type -> ZM (String,[String])
minMaxType cmp t1 t2 = do
  (z1,c1) <- convType t1
  (z2,c2) <- convType t2
  return (app "ite" [app cmp [z1,z2],z1,z2],c1 ++ c2)

unknownType :: Type -> ZM (String,[String])
unknownType t = do
  cache <- gets typeExprMap
  case M.lookup t cache of
    Just z -> return z
    Nothing -> do
      s <- get
      let name = "__bsc_num_" ++ show (nextId s)
          constraint = app ">=" [name,"0"]
      put s { nextId = nextId s + 1,
              declarations = M.insert name () (declarations s) }
      return (name,[constraint])



-- -------------------------
-- SMT-LIB rendering

app :: String -> [String] -> String
app op args = "(" ++ unwords (op:args) ++ ")"

andAll :: [String] -> String
andAll [] = "true"
andAll [x] = x
andAll xs = app "and" xs
