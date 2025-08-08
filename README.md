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

# tmux-rs

A rust port of [tmux](https://github.com/tmux/tmux).

## Why?

Why not? This a fun hobby project for me. It's been my gardening for the past year.

Why not just use [zellij](https://zellij.dev/)? I like tmux. I want tmux,
not something else.

## Installation

Like `tmux`, it requires `libevent2` and `libtinfo` (usually packaged with ncurses).

For Linux

```sh
sudo apt-get install libncurses-dev libevent-dev
cargo install tmux-rs
tmux-rs
```

For OSX
```sh
brew install libevent ncurses pkg-config

# Set  platform specific path overrides 
export PKG_CONFIG_PATH="$(brew --prefix)/lib/pkgconfig"
export LIBRARY_PATH="$(brew --prefix)/lib"
export CPATH="$(brew --prefix)/include"

cargo install tmux-rs
tmux-rs
```

Also don't run `tmux-rs` if there is an existing tmux session running in
the background. It could result in crashes of the original tmux session.
