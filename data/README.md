# Data Sources

[earthbyte.org Grids](https://www.earthbyte.org/webdav/ftp/earthbyte/agegrid/2020/Grids/)

From the `readme.txt`

```
age.2020.1.GTS2012.1m.nc    

	NetCDF-4 formatted file (compatible with GMT5 and above).
	The geographic grid spans longitudes from -180 to
	+180 and latitudes from 90 N to -90 N.
	Z value is the age of oceanic crust (Myrs).
	1 minute resolution, gridline-registered.
	Timescale used is (Ogg 2012).
```

The library we are using to read this data is only compatible with NetCDF-3, so the above file(s) have been converted to this format and renamed as `*.classic.nc`. The `scripts/netcdf4to3.sh` was used in the conversion.
