module Test.Main where

import Prelude

import Effect (Effect)
import Effect.Aff (launchAff_)
import Effect.Class (liftEffect)
import Test.BasicsSpec as BasicsSpec
import Test.ErrorsSpec as ErrorsSpec
import Test.GenericsSpec as GenericsSpec
import Test.Spec.Reporter (consoleReporter)
import Test.Spec.Runner.Node (runSpecAndExitProcess)
import Test.WriteViaSpec as WriteViaSpec

-- Native binaries cannot discover modules dynamically, so the specs are
-- listed explicitly instead of using `Test.Spec.Discovery.discover`.
main ∷ Effect Unit
main = launchAff_ $
  liftEffect $ runSpecAndExitProcess [ consoleReporter ]
    (BasicsSpec.spec *> GenericsSpec.spec *> ErrorsSpec.spec *> WriteViaSpec.spec)
