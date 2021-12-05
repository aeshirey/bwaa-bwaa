# bwaa-bwaa
An easy-to-use, standalone music server for your browser

Inspired by [dStream](https://github.com/DusteDdk/dstream) [on Hacker News](https://news.ycombinator.com/item?id=28910368), I wanted to write my own super easy-to-use server that would serve up music from my personal audio collection. I already use Plex but wanted a way to get my music more quickly, without logging in, and much lighter-weight interface.

![Screenshot of bwaa-bwaa web interface](https://i.imgur.com/HFCfr2e.png)

Instead of using dStream, I went (and continue to go) through more effort to build my own thing because I wanted to avoid using Docker, and I prefer to improve my Rust skills over spending time with much JavaScript. The current implementation is likely much less sophisticated than dStream is, so there's that.

## Features
- Can play MP3 files.
- UI could be worse

## TODO:
- [ ] Handle more audio file formats (flac, ogg, wav)
- [ ] Ability to rescan the input
- [ ] Search: Fuzzy w/relevance score
- [ ] Search: Handle diacritics
- [ ] Search: Pagination?
- [ ] UI: Clear search results easily
- [ ] UI: Sort results
- [ ] UI: Cleanup
- [ ] UI: Playlist, shuffle, etc.
