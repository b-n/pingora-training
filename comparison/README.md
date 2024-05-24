# Comparisons of pingora to nginx

This directory contains a number of nginx configuration files to illustrate the equivilant configuration.

These files assume the current user is running nginx (normally it would be it's own user), so you may run into some permission errors.

## Notes 

Step 4 - Config/CLI - is specific to the router (e.g. nginx has it's own cli options too).
Step 5 - Prometheus metrics - is also not supported without additional modules and/or awk/sed scripts on access.log files.

## Running

-. Make sure nginx is currently stopped - `nginx -s quit`
-. Run with full path to config files `nginx -c <full path to .conf>`
-. Reload changes with `nginx -c <full path to .conf> -s reload`

## Permissions

For the sake of demo'ing, it was easier to just run nginx as the current user. As such, I needed to make sure the current user:group owned paths that nginx expects to write to.

> [!WARNING]
> Do not run these commands unless you know what you're doing...

```sh
sudo chown $USER:$GROUP /var/lib/nginx
sudo chown -R $USER:$GROUP /var/log/nginx
```
