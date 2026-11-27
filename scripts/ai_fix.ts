/**
 * Reconciles this plugin with a new tidy release using OpenAI Codex, in two
 * stages that each run Codex in a session of its own:
 *
 *   1. a Codex agentic session edits the source and runs the checks until they
 *      pass (fixing breakage and/or syncing the schema and file matching), then
 *   2. a SEPARATE reviewer session reviews the result with Codex, so it can
 *      investigate (read the upstream tidy source, grep the repo). It is
 *      instructed to review only and not modify anything. If it finds blocking
 *      issues they are fed back to the stage-1 Codex for another pass, bounded
 *      by `REVIEW_MAX_ROUNDS`. If it still isn't approved, this throws so the
 *      workflow fails and nothing is published.
 *
 * Two situations call this:
 *   - a patch bump that failed the checks (fix the breakage), and
 *   - any minor bump (review for new/renamed/removed options and files even
 *     when it still compiles).
 *
 * Both stages use the default model of the Codex CLI, which is installed at
 * its latest version when it is missing, so they move to a new model when
 * Codex does. Set `CODEX_MODEL` or `REVIEW_MODEL` to pin the model of a stage.
 *
 * Codex must NOT commit or push -- `update.ts` captures the working tree
 * changes into the existing tidy bump commit afterwards.
 */
import { $ } from "automation";

export interface AiFixOptions {
  /** true for a patch bump, false for a minor/major bump. */
  isPatchBump: boolean;
  /** tidy version currently in Cargo.toml (before the bump). */
  fromVersion: string;
  /** tidy version being upgraded to. */
  toVersion: string;
  /** Whether the checks (test + clippy + wasm build) already passed. */
  checksPassed: boolean;
  /** Combined output of the failing checks (empty when `checksPassed`). */
  checkOutput: string;
}

/** Max number of times the reviewer may send changes back to Codex. */
const REVIEW_MAX_ROUNDS = 2;

export async function aiFixTidyUpdate(options: AiFixOptions): Promise<void> {
  const apiKey = requireApiKey();
  await ensureCodexInstalled();
  await codexLogin(apiKey);

  // stage 1: let Codex reconcile the update.
  await runCodex(buildFixPrompt(options));

  // stage 2: independent second-model review, with a bounded refix loop.
  for (let round = 1;; round++) {
    const review = await reviewChanges(options);
    logReview(review);
    if (review.approved) {
      return;
    }
    if (round > REVIEW_MAX_ROUNDS) {
      const blocking = review.issues.filter((i) => i.severity === "blocking");
      throw new Error(
        `AI reviewer did not approve after ${REVIEW_MAX_ROUNDS} refix round(s). Blocking issues:\n`
          + blocking.map((i) => `  - ${i.description}`).join("\n"),
      );
    }
    $.logStep(`Reviewer requested changes — Codex refix round ${round}...`);
    await runCodex(buildRefixPrompt(review));
  }
}

// stage 1: Codex ---------------------------------------------------------------

async function runCodex(prompt: string): Promise<void> {
  const args = ["exec", "--dangerously-bypass-approvals-and-sandbox", "--skip-git-repo-check"];
  const model = Deno.env.get("CODEX_MODEL");
  if (model) {
    args.push("--model", model);
  }
  args.push(prompt);

  $.logStep("Running Codex...");
  await $`codex ${args}`;
}

function buildFixPrompt(options: AiFixOptions): string {
  const { isPatchBump, fromVersion, toVersion, checksPassed, checkOutput } = options;

  const situation = checksPassed
    ? `tidy (the \`tidy\` crate from lukeed/tidy) was upgraded from ${fromVersion} to ${toVersion} (a ${
      isPatchBump ? "patch" : "minor"
    } bump). The project already compiles and the checks pass, but the new tidy version may have ADDED, RENAMED, or REMOVED options, sections, or the files it formats, which this plugin's schema and file matching should reflect.`
    : `tidy (the \`tidy\` crate from lukeed/tidy) was upgraded from ${fromVersion} to ${toVersion} and the checks fail (the project no longer builds, or \`cargo test\` or \`cargo clippy\` fails, or a spec test's output changed). This is usually because an API this plugin uses changed (\`tidy::Formatter\`, \`tidy::Language\`, \`tidy::Error\`, \`tidy::config::Config\`, or the \`tidy::schema\` module), because tidy's options changed so \`deployment/schema.json\` no longer matches them, or because tidy now formats some code differently.`;

  const failureOutput = checkOutput.trim().length > 0
    ? [
      ``,
      `The checks currently fail with the output below. Use it as your starting point instead of re-running the checks just to rediscover the errors:`,
      "```",
      truncateHead(checkOutput.trim()),
      "```",
    ]
    : [];

  return [
    `You are updating the "dprint-plugin-tidy" Rust crate, a dprint plugin that wraps the \`tidy\` crate (lukeed/tidy, a formatter with Prettier's style for HTML, Vue, Handlebars, CSS, JSON, YAML, TOML, XML, GraphQL, INI, Dockerfiles, Markdown, JavaScript and TypeScript).`,
    ``,
    situation,
    ...failureOutput,
    ``,
    `Your goal: make the checks pass AND keep this plugin's schema and file matching in sync with tidy. Do NOT commit or push; only edit files in the working tree.`,
    ``,
    describeWiring(),
    ``,
    `To see exactly what changed in tidy, inspect the dependency source that cargo already downloaded, e.g.:`,
    `  find ~/.cargo/registry/src -maxdepth 2 -type d -name 'tidy-*'`,
    `then read \`tidy-*/src/schema.rs\` (the options and their defaults), \`tidy-*/src/config.rs\` (how a config is read), \`tidy-*/src/options.rs\` (\`Language::file\`), and the \`NAMES\`/\`EXTENSIONS\`/\`JS\`/\`TS\` lists and \`is_*_path\`/\`of_path\` functions in \`tidy-*/src/*/mod.rs\`, and diff them against the directory of the previous version if it is there too.`,
    ``,
    `Rules:`,
    `1. Options are passed through to tidy, so a new tidy option works without a Rust change. Keep \`deployment/schema.json\` matching \`tidy::schema\` (the test \`should_have_schema_matching_tidys_options\` checks the names and defaults): add a property for each new option with tidy's description and default (an option that needs \`js.sortImports\` gets no \`default\`, and its default goes in its description instead), list it in the right \`x-sections\` entry, and remove the properties of removed options. A new section also needs a \`<section>.enabled\` property, a section in \`x-sections\`, and an entry in \`SECTION_FILES\` in \`src/file_kind.rs\`.`,
    `2. If tidy now formats files it didn't (a new extension or file name in its lists), add them to \`SECTION_FILES\` in \`src/file_kind.rs\` under the section that formats them, and remove the ones it no longer formats. dprint only matches the last extension of a file and whole file names, so leave out multi-part extensions (ex. \`.json.example\`) and names tidy only matches with a suffix. The tests in \`src/file_kind.rs\` check every listed entry against \`tidy::Language::file\`.`,
    `3. Add or extend a spec test under \`tests/specs/\` for a new option or language.`,
    `4. These are this plugin's own and have no counterpart in tidy, so do NOT remove them because they are missing upstream: the \`<section>.enabled\` properties, the mapping of dprint's global \`lineWidth\`/\`indentWidth\`/\`useTabs\`/\`newLineKind\` onto tidy's \`width\`/\`indent\`/\`tabs\`/\`eol\` in \`src/configuration/resolve_config.rs\`, the Wasm stack size in \`build.rs\`, and the CRLF conversion of Markdown in \`src/format_text.rs\` (only remove that if tidy's Markdown options gained an \`eol\` that tidy honors).`,
    `5. If a spec test fails only because tidy intentionally formats some code differently now, update that spec's expected output and explain what changed in your final message. Never change an expected output to hide a bug in this plugin.`,
    `6. Do NOT edit \`README.md\`. Its config documentation is maintained separately, so leave it untouched.`,
    `7. Preserve the existing code style. Keep non-test code above test modules. New comments start lowercase unless multiple sentences.`,
    `8. When done, ALL of these must pass (CI denies clippy warnings, and the wasm build is what actually ships) — iterate until they are all clean:`,
    `     cargo test`,
    `     cargo clippy --all-targets --all-features -- -D warnings`,
    `     cargo build --target wasm32-unknown-unknown --features wasm --release`,
    `9. Do not change the plugin's own version in Cargo.toml, do not run git commit, and do not push.`,
    ``,
    `Spec test format: each file in \`tests/specs/\` starts with a \`-- <file name> --\` line that decides the language, then an optional \`~~ {"option": value} ~~\` config line, then \`== short description ==\` (\`!! short description !!\` in Markdown files), the input code, a line containing \`[expect]\`, and the expected formatted output. Follow the existing files as examples and the \`Config<Name>.txt\` naming, and run \`cargo test\` to confirm they pass.`,
  ].join("\n");
}

function buildRefixPrompt(review: ReviewResult): string {
  return [
    `An independent reviewer examined your changes to dprint-plugin-tidy and found issues that must be fixed. Address every blocking issue below, keeping \`cargo test\` green. Do not commit or push.`,
    ``,
    `Reviewer summary: ${review.summary}`,
    ``,
    `Issues:`,
    ...review.issues.map((i) => `  - [${i.severity}] ${i.description}`),
    ``,
    describeWiring(),
  ].join("\n");
}

function describeWiring(): string {
  return [
    `How the plugin is wired (keep all of these consistent with each other):`,
    `- \`src/configuration/resolve_config.rs\` -> turns the dprint config (dotted keys like \`js.semi\`, or section objects like \`"js": { "semi": false }\`) into the object a \`tidy.json\` has, with the dprint global config as tidy's shared options, and has tidy read it with \`tidy::config::Config::from_value\`, which reports invalid options. \`<section>.enabled\` is handled by the plugin and never reaches tidy. The section names come from \`tidy::schema::SECTIONS\`.`,
    `- \`src/configuration/configuration.rs\` -> the resolved \`Configuration\`, holding the \`tidy::Options\` (and those for CRLF files with \`newLineKind: "auto"\`).`,
    `- \`src/file_kind.rs\` -> \`SECTION_FILES\`, the extensions and file names dprint sends to the plugin per section, and \`language_of\`, which classifies a file with \`tidy::Language::file\`.`,
    `- \`src/format_text.rs\` -> formats with \`tidy::Formatter::format\` and turns a \`tidy::Error\` into a \`FormatError\`.`,
    `- \`deployment/schema.json\` -> the JSON schema of the config options shown to users, with tidy's options flattened to dotted keys. \`tests/test.rs\` checks it against \`tidy::schema\`.`,
    `- \`tests/specs/*.txt\` -> spec tests (run by \`dprint_development::run_specs\` via \`tests/test.rs\`).`,
  ].join("\n");
}

// keep the tail of long check output -- the errors and the "could not compile"
// summary are at the end, which is the most useful part for the AI.
function truncateHead(text: string, max = 20_000): string {
  return text.length > max ? `... (truncated)\n${text.slice(text.length - max)}` : text;
}

// stage 2: independent reviewer ------------------------------------------------

interface ReviewResult {
  approved: boolean;
  summary: string;
  issues: ReviewIssue[];
}

interface ReviewIssue {
  severity: "blocking" | "nit";
  description: string;
}

// The reviewer runs Codex too so it can actually investigate (read the tidy
// source cargo downloaded, grep the repo, verify option names against the real
// upstream API). Its verdict is captured as JSON via --output-last-message.
//
// NOTE: we deliberately do NOT use `--sandbox read-only` here. That makes Codex
// wrap commands in bubblewrap, which cannot set up its network namespace on the
// GitHub Actions runner ("bwrap: loopback: Failed RTM_NEWADDR: Operation not
// permitted"), so every command the reviewer runs fails before executing. We
// use the same no-sandbox mode as the fixer and enforce read-only via the
// prompt instead. (CI is itself a throwaway VM, and the final check gate in
// update.ts re-verifies whatever ends up in the working tree.)
async function reviewChanges(options: AiFixOptions): Promise<ReviewResult> {
  // the reviewer starts without the context of the fixer, which is what makes
  // it a second opinion. It uses the default model of Codex unless one is set.
  const model = Deno.env.get("REVIEW_MODEL");

  // record intent-to-add so any files Codex created also show in `git diff`.
  await $`git add -N .`.quiet();

  const outputFile = await Deno.makeTempFile({ prefix: "codex-review-", suffix: ".json" });
  try {
    $.logStep(`Reviewing changes with Codex (${model ?? "default model"})...`);
    const args = ["exec", "--skip-git-repo-check", "--dangerously-bypass-approvals-and-sandbox"];
    if (model) {
      args.push("--model", model);
    }
    args.push("--output-last-message", outputFile, buildReviewPrompt(options));
    await $`codex ${args}`;
    return parseReview(await Deno.readTextFile(outputFile));
  } finally {
    await Deno.remove(outputFile).catch(() => {});
  }
}

function buildReviewPrompt(options: AiFixOptions): string {
  return [
    `You are an independent reviewer for dprint-plugin-tidy, a dprint plugin that wraps the tidy crate (lukeed/tidy, a formatter with Prettier's style for many languages). tidy was just upgraded from ${options.fromVersion} to ${options.toVersion} and another AI edited this plugin to reconcile it. Review the UNCOMMITTED working-tree changes.`,
    `IMPORTANT: this is REVIEW ONLY. Read any files and run read-only commands (git diff, cat, grep, find, etc.) to investigate, but you MUST NOT edit, create, or delete any files, and MUST NOT run git commit or git push. Leave the working tree exactly as you found it.`,
    ``,
    `Investigate as needed:`,
    `- Run \`git --no-pager diff\` (and \`git status\`) to see exactly what changed, including any new files.`,
    `- Verify against the REAL tidy ${options.toVersion} source cargo downloaded:`,
    `    find ~/.cargo/registry/src -maxdepth 2 -type d -name 'tidy-*'`,
    `  then read \`tidy-*/src/schema.rs\`, \`tidy-*/src/config.rs\`, \`tidy-*/src/options.rs\`, and the file name and extension lists in \`tidy-*/src/*/mod.rs\`.`,
    `- Read the plugin files to confirm they are consistent with each other and with upstream.`,
    ``,
    describeWiring(),
    ``,
    `Verify specifically:`,
    `- \`deployment/schema.json\` has a property for every tidy option (dotted for section options) with tidy's description and default, each in exactly one \`x-sections\` entry, and none for removed options.`,
    `- Every extension and file name tidy newly formats is in \`SECTION_FILES\` in \`src/file_kind.rs\` under the right section (except multi-part extensions and suffixed names, which dprint can't match), and none it stopped formatting are left. A missed one is a blocking issue.`,
    `- A new option or language has a spec test under \`tests/specs/\`.`,
    `- The \`<section>.enabled\` properties, the mapping of the dprint global config, the Wasm stack size in \`build.rs\`, and the Markdown CRLF conversion were not removed (they are this plugin's own), unless tidy's Markdown now honors an \`eol\` option.`,
    `- Any changed expected output in \`tests/specs/\` is explained by an intentional formatting change in tidy ${options.toVersion}, not by a bug in this plugin. An unexplained change is a blocking issue.`,
    `- README.md was NOT modified (its documentation is maintained separately); flag any README.md change as a blocking issue.`,
    `- No obvious correctness bugs, and code style matches the surrounding code.`,
    ``,
    `When done, your FINAL message must be ONLY a JSON object (no markdown code fences, no extra prose) of exactly this shape:`,
    `{"approved": true|false, "summary": "one sentence", "issues": [{"severity": "blocking"|"nit", "description": "..."}]}`,
    `Set approved=false if there is any blocking issue; nits alone do not block.`,
  ].join("\n");
}

function parseReview(message: string): ReviewResult {
  const review = JSON.parse(extractJsonObject(message)) as ReviewResult;
  if (typeof review.approved !== "boolean" || !Array.isArray(review.issues)) {
    throw new Error(`Codex review returned malformed JSON:\n${message}`);
  }
  return review;
}

// Codex is instructed to emit only JSON, but tolerate an accidental code fence
// or surrounding prose by extracting the outermost { ... } object.
function extractJsonObject(text: string): string {
  const start = text.indexOf("{");
  const end = text.lastIndexOf("}");
  if (start === -1 || end === -1 || end < start) {
    throw new Error(`Could not find a JSON object in Codex review output:\n${text}`);
  }
  return text.slice(start, end + 1);
}

function logReview(review: ReviewResult): void {
  $.log(`Review: ${review.approved ? "approved" : "changes requested"} — ${review.summary}`);
  for (const issue of review.issues) {
    $.logLight(`  [${issue.severity}] ${issue.description}`);
  }
}

// setup ------------------------------------------------------------------------

function requireApiKey(): string {
  const apiKey = Deno.env.get("OPENAI_API_KEY");
  if (!apiKey) {
    throw new Error("OPENAI_API_KEY is not set. It is required to run the AI fixer and reviewer.");
  }
  return apiKey;
}

async function ensureCodexInstalled(): Promise<void> {
  const found = await $`codex --version`.noThrow().quiet();
  if (found.code === 0) {
    return;
  }
  $.logStep("Installing OpenAI Codex CLI...");
  await $`npm install -g @openai/codex`;
}

// `codex exec` does not read OPENAI_API_KEY on its own -- it needs credentials
// stored via `codex login` first, otherwise its requests go out with no auth
// header and the API returns 401. The key is piped over stdin so it never
// appears in the process arguments.
async function codexLogin(apiKey: string): Promise<void> {
  $.logStep("Authenticating Codex with the OpenAI API key...");
  await $`codex login --with-api-key`.stdinText(apiKey);
}
