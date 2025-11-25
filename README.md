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

### Pricing Data
- https://api.warframestat.us/wfinfo/prices (Platinum)
- https://api.warframestat.us/wfinfo/filtered_items (Ducats)


### Project Structure

* `./lib` is is the base and can be used in other projects
* `./bin` is shared code between binaries
* `./cli` is code for cli binary
* `./gui` is code for gui binary (not yet implement)
* `./overlay` is code for overlay stuff, this will probably be in its own crate for anyone to use
