# prelude-path

A source file whose path equals a prelude file's path is an error. (SPEC §2.1)

A path names one file of the project, and the vocabulary's prelude already holds a file at that path.

```sym-error at-prelude-path
Clip c:
    Note/60
```

The source file stands at `prelude.sym`, the path of the prelude's file.

```sym
Clip c:
    Note/60
```
