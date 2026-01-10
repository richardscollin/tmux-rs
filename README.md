<section class="warning">

> [!WARNING]
> This project is alpha quality and has many known bugs. It's written in
> almost entirely unsafe Rust. Don't use it yet unless you're willing to deal
> with frequent crashes.
>
> THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
> WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
> MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
> ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
> WHATSOEVER RESULTING FROM LOSS OF MIND, USE, DATA OR PROFITS, WHETHER
> IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING
> OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

</section>

# Notes for 1.85 Compat Branch

The purpose of this branch is to be able to build on platforms which only ship older rust compilers.

The platforms I care about specifically are Debian 13 and termux.

This branch will be periodically rebased on top of the main branch.
If you care about it's git history, I recommend you fork this repo,
because it will otherwise be rewritten during rebasing.

I don't care about it linting cleanly on clippy either.

# tmux-rs

A rust port of [tmux](https://github.com/tmux/tmux).

## Why?

Why not? This a fun hobby project for me. It's been my gardening for the past year.

Why not just use [zellij](https://zellij.dev/)? I like tmux. I want tmux,
not something else.

## Installation

### Linux

Like `tmux`, it requires `libevent2` and `libtinfo` (usually packaged with ncurses).

```sh
sudo apt-get install libncurses-dev libevent-dev
cargo install tmux-rs
tmux-rs
```

### macOS

```sh
brew install libevent ncurses
cargo install tmux-rs
tmux-rs
```
