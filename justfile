mod example
mod rust 'crates'

default: example::default

swift-format: example::swift-format
swift-format-check: example::swift-format-check

clean: 
  just example::clean 
  rm -rf target

test:
  just rust::test
  just example::rust::unit-test
  just example::rust::bindgen-test
  just example::apple::build-package-test
  just example::android::build-package-test