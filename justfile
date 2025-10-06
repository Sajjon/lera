mod example

# Default recipe (runs when you just type `just`)
default: example::default

swift-format: example::swift-format
swift-format-check: example::swift-format-check

clean: 
  just example::clean 
  rm -rf target
