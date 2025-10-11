mod example
mod rust 'crates'

default: example::default

swift-format: example::swift-format
swift-format-check: example::swift-format-check

clean: 
  just example::clean 
  rm -rf target
