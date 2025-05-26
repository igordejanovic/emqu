# emqu

A little CLI utility to embed/query text files.

# Usage

## Generate embeddings

```sh
emqu embed 'issues/*.txt' lionweb_emqu_issues.json
```

## Query

```sh
emqu query -t 5 lionweb_emqu_issues.json "What is the difference between key and name?"
```

# Motivation

A simple RAG for [gptel](https://github.com/karthink/gptel).

For example, to bring issues from the [LionWeb spec
project](https://github.com/LionWeb-io/specification/) into limited context:

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
2. Create embeddings as shown above.

3. Make a gptel tool which will be called to bring relevant issues into context.

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

