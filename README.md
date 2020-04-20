# Cloud Stratum Proxy
_a simple, cross-platform, multi-client Stratum SSL TCP proxy with SNI support_

`cloud-stratum-proxy` is a cross-platform, multi-client TCP proxy written in rust, that is designed for those "one-time" tasks where you usually end up spending more time installing a proxy server and setting up the myriad configuration files and options than you do actually using it.

## Usage

`cloud-stratum-proxy` is a command-line application. One instance of `cloud-stratum-proxy` should be started for each remote endpoint you wish to proxy data to/from. All configuration is done via command-line arguments, in keeping with the spirit of this project.

```
cloud-stratum-proxy [-b BIND_ADDR] -l LOCAL_PORT -h STRATUM_HOST

Options:
    -l, --local-port LOCAL_PORT
                        The local port to which cloud-stratum-proxy should bind to
    -h, --host STRATUM_HOST
                        The remote stratum server to which mining work will be forwarded.
    -b, --bind BIND_ADDR
                        The address on which to listen for incoming requests
    -d, --debug         Enable debug mode
```

Where possible, sane defaults for arguments are provided automatically.

## License

`cloud-stratum-proxy` is open source and licensed under the terms of the MIT public license.
