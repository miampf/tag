# Tag

A simple program to organize plain text files with tags.

## Installation

### As a binary

Simply head to the [releases](https://github.com/miampf/tag/releases) page and download the latest release for your system.

### From source

Simply clone the repository and run `cargo install --target=<YOURTARGET>` to build and install the project. Replace `<YOURTARGET>` with either `x86_64-pc-windows-gnu` for windows or `x86_64-unknown-linux-gnu` for linux.

## Usage

To use tag you must add a taglist as the first line of your text files. A tagline looks like the following:

```
tags: [#tag1 #tag2]
```

The tags in the tagline start with a `#` followed by letters (some none-ASCII letters are also supported), numbers, `_` and `-`. This tagline **must** be the first line of your file and **must not** be larger than one line. You can find the tagline grammar under [tagline.pest](./tagline.pest).

Once you've added taglines to your local files you can run `tag`. `tag` will search all subdirectories of a given directory and check if tagged files match your search query.

The `tag` help message:

```
Usage: tag [OPTIONS] <QUERY> <PATH>

Arguments:
  <QUERY>  Search query for the tags
  <PATH>   The path that will be searched

Options:
  -s, --silent   Only print the paths of matched files
  -h, --help     Print help
  -V, --version  Print version
  ```

A query contains operators and tags. Usable operators are `&` for the logical AND, `|` for the logical OR and `!` as a unary NOT. Furthermore, you can nest queries by using parantheses. A query could look like this:

```
#tag1 & #tag2 | (!#tag3 & #tag4)
```

This query would match all files that contain `#tag1` AND `#tag2` OR files that don't contain `#tag3` while also containing `#tag4`. You can find the query grammar under [query.pest](./query.pest).
