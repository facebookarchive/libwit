# libwit

*This library is community-maintained and likely broken. Please use the HTTP API or the Node.js/Python/Ruby SDKs for non-experimental needs (https://wit.ai/docs). We gladly accept pull requests*

`libwit` is a small library that makes it easy to integrate Wit.ai with many programming languages. It manages client audio recording and communication with Wit.ai.

To compile, make sure you have `autoconf` installed. Then run:

```bash
cargo build
```

This will create `libwit-******.rlib` and `libwit-******.a` files in the `target` folder. The first one can be linked as a normal C library. Depending on your platform, the build command will also tell you which additional libraries you will need to link to your program.

To compile the example, run:

```bash
mv build/libwit-******.a libwit.a
cd example
gcc -Wall -o test test.c -I ../include -L . -lwit <additional libraries>
```

Make sure to replace `libwit-******.a` with the exact name of the file created with `cargo build`.
The additional libraries in the `gcc` command are those shown in the output of `cargo build`.
