# internal-error

An internal error is a defect of the compiler, not of the program: a lookup inside the compiler missed what it expected, and the run draws this one diagnostic and fails.

The compiler names its own defect instead of reporting a wrong rule; report it with the program that drew it.
