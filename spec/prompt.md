# PCD Translation Prompt

## Environment

Input directory:  [YourInputDir]
Output directory: [YourOutputDir]

You have a bash tool. Use it for all file operations and build verification.
Do NOT print file contents to the terminal as a substitute for writing them to disk.

File writing pattern — use this for every source file:
```bash
cat > [YourOutputDir]filename.ext << 'EOF'
... file content ...
EOF
```

After writing each file, confirm it exists and is non-empty:
```bash
ls -la [YourOutputDir]/filename.ext
```

Do NOT write prompt.md, system_prompt.md, or any input file to the output directory.

---

## CRITICAL REQUIREMENTS

Read these before reading anything else. They override any conflicting
instruction in any other file.

1. **Read ALL hints files before writing any code.**
   The active MILESTONE in the spec has a `Hints-file:` field listing one or
   more hints files. Read every one of them from the input directory before
   producing a single line of code. They contain known failure modes from
   previous translation runs. Ignoring them guarantees repeating those failures.

2. **The production CommandRunner implementation MUST use the platform
   subprocess API.** It must actually execute system commands and capture
   their output. A stub that returns ("", "", nil) or equivalent for all
   inputs is not acceptable — it will cause every collection module to
   silently produce empty output while the build appears to succeed.

3. **render_human MUST contain exactly 30 separate typed functions**, one
   per row in the SECTION-MAP table in the spec. Do NOT write a single
   generic dispatcher function. A dispatcher that covers only some types
   silently drops all others — this was the failure mode in previous runs.

4. **All scope fields MUST be initialised to empty-but-valid objects, never
   null.** A null reference serialises to JSON `null`. An empty initialised
   scope serialises to `{"_attributes":{},"_elements":[]}`. Only the latter
   is schema-compatible. The hints file gives concrete examples for the
   target language.

5. **The active MILESTONE has `Scaffold: true`.** This means your only
   objective is a complete, compilable skeleton. Do not implement real
   collection or rendering logic. Stubs only. The compile gate is the
   sole acceptance criterion.

---

## Input files

The following files are in [YourInputDir]:

1. `cli-tool.template.md` — deployment template: conventions, constraints,
   delivery phases, compile gate, and deliverables table for this component type.

2. `sitar.md` — the component specification in PCD format.

3. Additional hints files named in the spec's active MILESTONE `Hints-file:`
   field. Read them before writing any code.

---

## Step 1 — Read before writing

Before producing any output, read these files in this order:

1. This prompt (you are reading it now)
2. `sitar.md` — find the active MILESTONE, note the `Hints-file:` field
3. Every hints file listed in `Hints-file:` — read them completely
4. `cli-tool.template.md` — read the EXECUTION section and DELIVERABLES table

Only after reading all four items may you begin writing files.

---

## Step 2 — Resolve target language

Read the TEMPLATE-TABLE in `cli-tool.template.md`. Find the LANGUAGE row.
The default language is stated there. Use it unless the spec's META section
declares a `Language:` override.

State the resolved language in the translation report. All subsequent
decisions (file extensions, build commands, dependency format, type
conventions) follow from this single resolved value.

---

## Step 3 — Active MILESTONE governs translation scope

Find the `## MILESTONE:` section in the spec with `Status: active`.
Exactly one milestone may be active. If zero or more than one are active,
halt and report the error.

**If `Scaffold: true` (current state):**

Your objective is a complete compilable skeleton of the ENTIRE component.
Read the full spec to understand all types, interfaces, and function
signatures. Then:

- Create all source files the completed implementation will ever need
- Define all types, structs, enums, and interfaces from the spec TYPES
  and INTERFACES sections
- Write a stub body for every function declared by every BEHAVIOR in the spec
- Every stub: correct signature, correct zero-value return, silent at
  normal verbosity (no "not implemented" messages during normal runs)
- Do NOT implement any real collection or rendering logic
- The compile gate is the only acceptance criterion

After this pass, all subsequent milestone translators replace stub bodies
only. They never create new files or new types.

**If `Scaffold: false` (future milestones):**

- Implement only BEHAVIORs listed under `Included BEHAVIORs:`
- Leave all `Deferred BEHAVIORs:` stubs exactly as they are
- Do not modify any other file or function body

---

## Step 4 — Stub contract

Every stub function must:

1. Have the **correct signature** — parameters and return types matching
   what the caller declared in the spec INTERFACES or BEHAVIOR blocks
2. Return the **correct zero value** for its output type:
   - Collection functions returning a scope object: return an initialised
     empty scope with empty attributes map and empty elements list —
     NEVER a null/nil/None reference
   - Functions returning a string: return empty string ""
   - Functions returning (string, error) or equivalent: return ("", nil)
   - Functions returning bool: return false
3. Be **silent at normal verbosity** — no output to stderr unless a
   debug environment variable is set (see hints file for the variable name)
4. **Compile cleanly** — no unused imports, no type errors

The hints file for the target language gives concrete code examples of
correct stub bodies.

---

## Step 5 — INTERFACES section

If the spec contains an `## INTERFACES` section, produce every declared
implementation:

- All production implementations (OSFilesystem, OSCommandRunner, all
  Renderer variants, all PackageBackend variants)
- All test doubles (FakeFilesystem, FakeCommandRunner, FakeRenderer,
  FakePackageBackend)

The production CommandRunner is NOT a stub. See CRITICAL REQUIREMENT 2.
All Renderer production implementations are stubs in M0 but must be present.

---

## Step 6 — Deliverables

Read the DELIVERABLES table in `cli-tool.template.md`. Produce every file
marked `required`. Note which files are `supported` (only if preset activates
them) and which are `forbidden`.

Do not invent filenames. Map every deliverable to the concrete filename given
in the template's naming convention using `<n>` = `sitar`.

Write files in the delivery order specified in the template EXECUTION section.
After writing each file, verify it with `ls -la`.

---

## Step 7 — Compile gate

After all source files are written, run the compile gate appropriate for
the resolved language. For the template default language:

```bash
cd /tmp/pcd-ollama-output
<build command from template EXECUTION section>
```

If compilation fails:
- Read the error output carefully
- Fix only the file(s) containing the reported errors
- Do not rewrite unaffected files
- Re-run the build
- Repeat until the build passes or all reasonable fixes are exhausted

Record pass/fail in the translation report.

Do NOT skip the compile gate. Do NOT claim it passed without running it.
If your environment cannot run the build tool, state this explicitly in
the translation report under "Compile gate not executed" and explain why.

---

## Step 8 — Translation report

Write `TRANSLATION_REPORT.md` LAST, after all other files are written and
the compile gate has passed.

The report must cover:

- Target language resolved and how (template default, spec override, or preset)
- Hints files read (list each filename and confirm it was read before coding)
- Active MILESTONE: name, Scaffold value, Included BEHAVIORs, Deferred BEHAVIORs
- Stub functions produced: count and list
- Compile gate result: pass or fail, with exact command run
- Deviations from the spec: any BEHAVIOR not implemented exactly as written,
  with reason
- Specification ambiguities: any ambiguous instruction, and the conservative
  interpretation chosen
- Per-example confidence table:

  | EXAMPLE | Confidence | Verification method | Unverified claims |

  Confidence levels:
  - High   = verified by a named test that passes without a live system
  - Medium = partially tested; some paths require a live system
  - Low    = code review only; not testable without a live privileged system

  For M0: most examples will be Low confidence. This is correct and expected.
  Do not inflate confidence. An honest Low is more useful than a false High.

---

## What NOT to do

- Do NOT output file contents to the terminal instead of writing them to disk
- Do NOT skip the compile gate or claim it passed without running it
- Do NOT stub OSCommandRunner.Run to return empty strings for all inputs
- Do NOT write a single generic render dispatcher instead of 30 typed functions
- Do NOT initialise scope fields to null/nil/None
- Do NOT implement format=yast2 (it is removed from the spec)
- Do NOT ask clarifying questions — make the conservative interpretation,
  implement it, and note the ambiguity in the translation report
- Do NOT write prompt.md or any input file to the output directory
