# The owner's rulings on the compiler's design as a whole, word for word

Saved by the primary peer. Each ruling is dated and says how it reached the primary. The context around a ruling is the primary's or the supervisor's words, never the owner's; the owner's words stand alone in quotation marks.

---

# First, 2026-11-15: the generated-Rust engine is the main mode

Relayed to the primary by the supervisor from the owner's chat on 2026-11-15. The question the supervisor put to the owner, in the supervisor's words: whether the generated-Rust engine of `ARCHITECTURE.md` §8 stays a goal, since the SPEC defines one meaning and no modes, ARCHITECTURE §4 says "The runtime has two engines over one seam: the node-by-node evaluator of this section, which serves script mode, and the generated Rust of §8, which a shipped program runs. Both compute every cell from the same Program, bit for bit, and the differential test of §8 holds them equal over every fixture", §8 names `symphonyscript::codegen::generate(sources, &vocab)` returning a Rust module with `pub fn program() -> Program` and one function per derived cell, shared body and `for` expression, the old code on `symphonyscript/r3` holds no generator, and the notes never named it. The primary read those sentences of ARCHITECTURE and the absence in the code before saving this.

The owner's words:

"it not only stays a goal, it is to be MAIN mode of the language, while interpreted mode stays as convenience secondary mode."

---

# Second, 2026-11-15: only settled things go in the SPEC

Relayed to the primary by the supervisor from the owner's chat on 2026-11-15. The question the supervisor had put to the owner, in the supervisor's words: whether SPEC §12.5 should gain a sentence naming the two engines and their bit-identical agreement.

The owner's words:

"nothing open or semi-open goes in the SPEC. only settled things go in the SPEC, then cold-reviewed under heavy scrutiny and finally committed and pushed. as you said compiled mode is not designed yet ."

---

# Third, 2026-11-15: no loops, tests or processes were approved

Relayed to the primary by the supervisor from the owner's chat on 2026-11-15. The context, in the supervisor's words: it had drafted a prompt for the agent that writes the agent files and told it the notes' Settled lines hold "the loops as I approved them".

The owner's words:

"I did not approve any loops, tests, processes or anything post STANDARD.md"
