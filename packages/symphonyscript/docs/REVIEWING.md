# Reviewing

How a SymphonyScript reviewer covers its lens, reviews the whole diff, reviews a fix round and reports. PLAN § Reviews decides the lenses, the shared build, the ratings and the speed targets; `SMELLS.md` and the general sections PLAN § Reviews names hold the defects to check for.

## How you work

- Read code by the function, and send independent reads and probes in one turn, because every turn costs latency and every whole file you read stays in your context to the end.
- Read whole only what you judge; for a fact, grep for it and read the lines it names, because a review that compacts loses minutes to re-reading its own work.

## Your lens first

- **Lens 1, SPEC conformance.** Walk every SPEC section the milestone touches sentence by sentence, probe a valid and a faulty program for each rule, and check the rule's row in `tests/error_rules.rs`, because a rule nobody probed is a rule nobody checked. Then list what the milestone adds against every consumer `SMELLS.md` § A construct that misses a consumer names, and probe each pair, because a defect in a pair nobody probes survives every review that reads only the diff.
- **Lens 2, adversarial programs and probes.** For each construct the milestone adds, generate programs over its forms, the operators, the positions and the declaration orders, cut every header and bracket at each token for one-fault programs, and check two-fault programs against their twins (ARCHITECTURE §7), because hand probes reach the forms their author imagines and a generator reaches the rest.
- **Lens 3, the hot path and the speed targets.** Run each walk on the hot path at three doubling sizes, in every shape its input takes (a chain, a ring, nesting, sharing, many readers of one value, a growing argument) and in forward and reversed declaration order, counting work, before you measure the targets, because a walk is linear in the shapes its test builds and can be quadratic in the rest.
- **Lens 4, code and test quality.** Read every changed function for the smells and the general patterns, every classifier for a catch-all arm, and every test for expected values from the documents, a class generated rather than listed, and a row that can fail, because a test that cannot fail passes the defect it was written for.

## Then your part of the diff

- **Sweep the class before you report.** On a finding, generate its class's programs over every dimension it varies and every declaration order, and report every member with the rule that decides them, because a review that reports one member of a class buys a round for each next member.
- **Apply every help.** Apply the code a help quotes and compile again, because a help that reads well can name an edit the compiler refuses.
- **Report a missing rule as a gap.** When no SPEC or ARCHITECTURE sentence decides a class's verdicts, report the class as a gap with a proposed rule, because verdicts decided a row at a time contradict one another.
- **State a fix as a property.** Propose the property the code must hold, the sentence that decides it, an adversarial row and the valid rows it keeps, and check each row against the SPEC, because a mechanism fixes the reported case and reaches the brief as a row nobody derived.
- **Name what the fix needs.** Say whether the property holds by repairing a mechanism the code already has, or needs a mechanism the code lacks, a new walk, key, reader, template or check, or a construct the milestone's steps do not name, and report the latter as a feature proposal with its rating, never as a fix for the round, because a feature built in a fix round is designed one gap at a time under reviews scoped to a diff, and each round's new machinery is the next round's Blocking finding.

## A warm review

- Check each fix of your own accepted findings: the reported case, then its class generated over every dimension, then its valid rows, then its cost at three sizes, because a fix proven on the reported form can leave the class's other forms.
- Review in full every line the round changed, `git diff <round start> <tip> -- packages/symphonyscript`, for correctness, the smells and the cost, because a fix changes every program its rule reaches.
- Report a Blocking defect you see as a finding of the round, and write everything else as a finding for the ledger, in the same form, because PLAN § Reviews keeps only Blocking defects in the round and sends the rest to the next milestone's cold review.

## Report

For each finding: its rating; where, by file and lines; the defect; the rule, quoting its SPEC or ARCHITECTURE sentence; the case, what it draws and what it should; the class, its dimensions and every member found; the fix as a property, with adversarial and valid rows, and whether it repairs a mechanism the code has or needs one the code lacks. Then, for lens 3, each measurement with its method; and each ledger entry the brief lists, with whether its case reproduces. Write findings, not narrative, because the head consolidates four reports into one brief.
