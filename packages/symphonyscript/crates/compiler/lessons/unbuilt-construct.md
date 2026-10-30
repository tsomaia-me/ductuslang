# unbuilt-construct

An unbuilt construct is one the parser reads and whose meaning this compiler does not build: the summary names the construct and the SPEC section that defines it, and the construct takes an error placeholder, which every later stage accepts without a diagnostic.

The compiler names the construct instead of reporting a rule the program does not break; write the program without it.
