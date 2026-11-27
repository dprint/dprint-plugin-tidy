# dprint-plugin-tidy

[![npm version](https://img.shields.io/npm/v/@dprint/tidy.svg)](https://www.npmjs.com/package/@dprint/tidy) [![CI](https://github.com/dprint/dprint-plugin-tidy/workflows/CI/badge.svg)](https://github.com/dprint/dprint-plugin-tidy/actions?query=workflow%3ACI)

Adapter for [tidy](https://github.com/lukeed/tidy) for use as a formatting plugin in [dprint](https://github.com/dprint/dprint). tidy formats in Prettier's style, and this plugin uses it for:

- HTML (and MJML), Vue and Handlebars
- CSS, SCSS and Less
- JSON, JSONC and JSON5
- YAML, TOML, XML (and SVG) and INI (and gitconfig)
- GraphQL
- Markdown
- Dockerfiles
- JavaScript, TypeScript and JSX

Code embedded in a file (ex. a `<script>` in HTML, or a code block in Markdown) is formatted by tidy too, when tidy knows its language.

## Install

[Install](https://dprint.dev/install/) and [setup](https://dprint.dev/setup/) dprint.

Then in your project's directory with a dprint.json file, run:

```shellsession
dprint add tidy
```

Note: You do not need tidy installed as dprint will run it from the .wasm file in a sandboxed environment.

tidy formats languages that other dprint plugins also format (ex. TypeScript, JSON and Markdown). When two plugins format the same file, the one listed first in the `plugins` array formats it. To leave a language to another plugin, turn it off here with `"<section>.enabled": false` (ex. `"markdown.enabled": false`).

## Configuration

To add configuration, specify a `"tidy"` key in your dprint.json:

```jsonc
{
  "tidy": {
    "width": 100,
    "quote": "single",
    "js.semi": false,
    "markdown.wrap": "always",
    "toml.enabled": false,
  },
  "plugins": [
    // ...etc...
  ],
}
```

The options and their defaults are the ones a [`tidy.json`](https://tidy.lukeed.com/docs/options/) has. The options of a language section are written with the section first and a dot (ex. `"js.semi": false`), or in an object named after the section, as in a `tidy.json` (ex. `"js": { "semi": false }`). An option every language has, like `width` or `quote`, can also be set for one section only (ex. `"markdown.width": 120`).

dprint's global `lineWidth`, `indentWidth`, `useTabs` and `newLineKind` are used as tidy's `width`, `indent`, `tabs` and `eol` when they're set, and tidy's defaults otherwise. With `"newLineKind": "auto"`, a file keeps the line endings it has.

Each section has an `enabled` option, which is `true` by default: `html`, `vue`, `handlebars`, `css`, `json`, `yaml`, `toml`, `xml`, `markdown`, `dockerfile`, `ini`, `graphql` and `js`.

A file with a syntax error is reported as an error rather than formatted, and range formatting isn't supported.

For an overview of the config, see https://github.com/dprint/dprint-plugin-tidy/blob/main/deployment/schema.json

## JS Formatting API

- [JS Formatter](https://github.com/dprint/js-formatter) - Browser/Deno and Node
- [npm package](https://www.npmjs.com/package/@dprint/tidy)

## Versioning

This repo automatically upgrades to the latest version of tidy once a day. You can check which version of tidy is being used by looking at the `tidy` entry in the Cargo.toml file in this repo:

https://github.com/dprint/dprint-plugin-tidy/blob/main/Cargo.toml

At the moment, the version of this plugin does not reflect the version of tidy.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details. tidy is licensed under the [MIT License](https://github.com/lukeed/tidy/blob/main/license) as well.
