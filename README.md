# Small World

An expansion tectonic model of the Earth

## Setup

This project was developed and tested on macOS.

This project uses GIT LFS. Install it. After cloning this repo run

```
git lfs pull
```

## Converting NetCDF to an Image

Run:

```
cd model
cargo run --release --bin nc_to_png
```

## Using the Viewer

Run:

```
npm start
```

and visit http://localhost:8080/
