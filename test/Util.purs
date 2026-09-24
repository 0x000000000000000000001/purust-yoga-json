module Test.Util where

import Prelude

import Data.Either (Either(..))
import Data.Semigroup.Foldable (intercalateMap)
import Effect.Aff (Aff)
import Foreign (Foreign)
import Test.Spec.Assertions (fail, shouldEqual)
import Type.Proxy (Proxy(..))
import Yoga.JSON (class ReadForeign, class WriteForeign, read, readJSON, write, writeJSON)
import Yoga.JSON.Error (renderHumanError, toJSONPath)

-- | A round-trip must decode back to the value it started from, not merely
-- | decode successfully.
roundtrips ∷ ∀ a. Show a ⇒ Eq a ⇒ ReadForeign a ⇒ WriteForeign a ⇒ a → Aff Unit
roundtrips x = do
  x # write # shouldRead (Proxy ∷ _ a) x
  x # writeJSON # shouldReadJSON (Proxy ∷ _ a) x

shouldRead ∷ ∀ a. Show a ⇒ Eq a ⇒ ReadForeign a ⇒ Proxy a → a → Foreign → Aff Unit
shouldRead _ expected = read >>> case _ of
  Left e → fail (intercalateMap "\n" renderHumanError e <> "\n" <> intercalateMap "\n" toJSONPath e)
  Right actual → actual `shouldEqual` expected

shouldReadJSON ∷ ∀ a. Show a ⇒ Eq a ⇒ ReadForeign a ⇒ Proxy a → a → String → Aff Unit
shouldReadJSON _ expected = readJSON >>> case _ of
  Left e → fail (intercalateMap "\n" renderHumanError e <> "\n" <> intercalateMap "\n" toJSONPath e)
  Right actual → actual `shouldEqual` expected
