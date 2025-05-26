# emqu

A little CLI utility to embed/query text files.

# Usage

## Generate embeddings

```sh
emqu embed 'issues/*.txt' issues_new.json
```

## Query

```sh
emqu query -t 4 issues_new.json "What is the difference between key and name?"
```
