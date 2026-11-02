# tab-indentation

Leading whitespace is spaces; a tab in it is an error. (SPEC §2.2)

Layout depends on columns, and a tab has no single width.

```sym-error
Plant p:
	Valve
```

The body is indented with a tab.

```sym
Plant p:
    Valve
```
