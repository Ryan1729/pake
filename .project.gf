[gdb]
path=./rust-gdb

[commands]
Compile pake=shell cargo b --bin pake --profile debugging
Run pake=file target/debugging/pake;run&