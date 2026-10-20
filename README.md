# WF Fissure Price

This project is based on
[wfinfo-ng](https://github.com/knoellle/wfinfo-ng)
since it has not been updated in over a year, 
decided to make a new project instead of a fork and 
just copy code when needed

I want this project to work more like
[WFinfo](https://github.com/WFCD/WFinfo)
(mainly just the overlay stuff)

## Project Status
Currently, this project only works on Wayland

### Running the CLI

```sh
cargo run --package cli --bin wffp-cli
```

The CLI registers `io.github.Ricky12Awesome.WFFissurePrice` as its portal
application ID before requesting global shortcuts. When no matching
desktop entry is installed, it creates one in `$XDG_DATA_HOME/applications`
(normally `~/.local/share/applications`). This also supports running from Cargo;
the entry points to the current executable, updates if it moves, and is hidden
from application menus. Existing entries installed by a user or package are kept.
The `--shortcut-id` option identifies the shortcut action, independently of the
application ID.
Registration also works with `GTK_USE_PORTAL=1`.

### Pricing Data
- https://api.warframestat.us/wfinfo/prices (Platinum)
- https://api.warframestat.us/wfinfo/filtered_items (Ducats)


### Project Structure

* `./lib` is is the base and can be used in other projects
* `./bin` is shared code between binaries
* `./cli` is code for cli binary
* `./gui` is code for gui binary (not yet implement)
* `./overlay` is code for overlay stuff, this will probably be in its own crate for anyone to use
