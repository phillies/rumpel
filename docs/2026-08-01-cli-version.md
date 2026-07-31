# Command-line version reporting

## Behavior

Running `rumpel --version` prints the package name and version in the form
`rumpel 0.3.0`, then exits successfully. It does not initialize GTK or
GStreamer and does not open a player window.

## Compatibility

The flag is additive and leaves existing file-opening and no-argument behavior
unchanged. The displayed version is compiled from Cargo package metadata, so it
always matches the built binary.