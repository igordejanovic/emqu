# `emqu`

A little CLI utility to chunk, embed, and query text files.

With this tool you can take a bunch of text sources (from PDFs, web, GitHub
issues, etc.), chunk it into semantically relevant parts, embed it into semantic
vector space, and then query it using natural language questions. Or, to put it
simply, this tool enables you to "talk to your documentation".

This enables building a simple [RAG
system](https://en.wikipedia.org/wiki/Retrieval-augmented_generation) which
solves problems of today's LLMs with limited context, where with each question a
relevant part of the documentation is found and fed into the context.


# Installation

```sh
cargo install --git https://github.com/igordejanovic/emqu
```

# Usage

## Semantic chunking

```emacs-lisp
emqu chunk 'lionweb-spec/*.adoc' context-chunked
```

## Generate embeddings

```sh
emqu embed 'context-chunked/*.adoc' lionweb_emqu_spec.json
```

## Query

```sh
emqu query -t 5 lionweb_emqu_spec.json "What is the difference between key and name?"
```

# Motivation

A simple RAG for [gptel](https://github.com/karthink/gptel) - a simple LLM client for Emacs.

For example, to bring issues from the [LionWeb spec project](https://github.com/LionWeb-io/specification/) into the limited context of LLMs:

1. Fetch GitHub issues:

    ```sh
    #!/bin/sh
    mkdir -p issues
    gh issue list -R LionWeb-io/specification --state all --limit 1000 \
    --json number,createdAt \
    | jq -r 'sort_by(.createdAt)[] | .number' \
    | xargs -I{} sh -c 'gh issue view {} -R LionWeb-io/specification \
        --json title,body,comments,number,author,createdAt --template \
        '"'"'Issue: #{{.number}} [{{.author.login}} ({{.createdAt}})]
    URL: https://github.com/LionWeb-io/specification/issues/{{.number}}
    Title: {{.title}}
    {{.body}}
    ----
    {{range .comments}}
    {{.author.login}} ({{.createdAt}}):
    {{.body}}
    ----
    {{end}}'"'"' > issues/{}.txt'
    ```

2. Create embeddings:

    ```sh
    emqu embed 'issues/*.txt' lionweb_emqu_issues.json
    ```

3. In Emacs, make a `gptel` tool to bring relevant issues into context:

    ```emacs-lisp
    (gptel-make-tool
    :name "lionweb_issues"
    :function (lambda (query)
                (let ((output (shell-command-to-string
                            (format "emqu query -t 5 /home/igor/docs/implementation/LionWeb/lionweb_emqu_issues.json \"%s\"" query))))
                (concat "Issues database results:\n" output)))
    :description "Search LionWeb issues database using emqu command"
    :args (list '(:name "query"
                :type string
                :description "Search query for the issues database"))
    :category "external")
    ```

Now you can ask questions about LionWeb in an Emacs Orgmode buffer keeping the
whole history of conversation as an Orgmode file with all the benefits of
Orgmode. Thanks to gptel, the conversation can be continued at any later time by
opening Org file.
