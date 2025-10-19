ipgrep
======

Search IP addresses and networks in text files.

*It's grep, except instead of regular expressions, the needle is an
IP-CIDR. The match-mode decides whether your CIDR is contained-in or
containing the haystack.*

|EXAMPLE|


--------
Examples
--------

*Find IPs or ranges in which your needle fits*::

    $ ipgrep 192.168.1.1,172.17.1.2 /etc/firejail/nolocal.net
    -A OUTPUT -d 192.168.0.0/16 -j DROP
    -A OUTPUT -d 172.16.0.0/12 -j DROP

*Find IPs contained inside your supplied ranges*::

    $ ip -br a | ipgrep 127.0.0.0/8 -m within
    lo               UNKNOWN        127.0.0.1/8 ::1/128

*Find exact IP matches*::

    $ ip -br a | ipgrep 0:0:0:0:0:0:0:1 -m equals
    lo               UNKNOWN        127.0.0.1/8 ::1/128

*Find and show only the matches*::

    $ ip -br a | ipgrep 127.0.0.1,::1 -o
    127.0.0.1/8
    ::1/128

*Supports IPv6*::

    $ ipgrep fe00::/7 /etc/hosts -m within
    fe00::0 ip6-localnet
    ff00::0 ip6-mcastprefix
    ff02::1 ip6-allnodes
    ff02::2 ip6-allrouters

*Looking for IPv4-mapped IPv6 addresses?*::

    $ echo ::ffff:127.0.0.1 | ipgrep ::ffff:0:0/96 -m w
    ::ffff:127.0.0.1


-----
Usage
-----

The following help was created before any development was done::

    ipgrep 0.1.0 - Search IP addresses and networks in text files

    Usage:
      ipgrep [OPTIONS] NEEDLES [HAYSTACK...]
      cat file.txt | ipgrep [OPTIONS] NEEDLES

    Needles are IPs or networks (e.g. 192.0.2.1, 2001:db8::/32).
    Multiple needles may be separated by commas or repeated.

    Haystacks are one or more files. If none given, stdin is read.

    The haystack lines may contain multiple IPs. These possible matches are
    referred to as items below.

    The options follow (closely mimicking grep options).

    Generic Program Information:
      --help                    Output a usage message and exit
      -V, --version             Print version and exit

    Matching Control:
      -a, --accept <MODE>       Accept input forms (may repeat or use commas):
                                  ip     - bare host IP
                                  net    - valid network (CIDR)
                                  oldnet - valid network (host/dotted-netmask)
                                  iface  - interface IP (host/mask)
                                  [default: ip,net,iface]
      -I, --interface-mode      Select interface IP matching mode (default: ip):
                                  ip       - treat as single IP
                                  net      - treat as if network bits were unset
                                  complain - complain/reject network bits
      -m, --match <TYPE>        Match mode (default: contains):
                                  contains - haystack net contains needle net
                                  within   - needle net contains haystack net
                                  equals   - exact IP or network equality
                                  overlaps - haystack and needle nets overlap

    General Output Control:
      -c, --count               Print only a count of matching items
      -l, --files-with-matches  List filenames with matches only
      -o, --only-matching       Print only the matching IPs/networks
      -q, --quiet               Quiet; exit status only

    Output Line Prefix Control:
      -h, --no-filename         Suppress filename prefix on output
      -n, --line-number         Prefix each output line (or item) with lineno
      -Z, --null                Output a zero byte instead of LF in output;
                                only useful in combination with -l

    Context Line Control:
      -A NUM, --after-context=NUM   Print NUM context lines before a match
      -B NUM, --before-context=NUM  Print NUM context lines after a match
      -C NUM, --context=NUM         Shorthand for -A NUM -B NUM

    File and Directory Selection:
      -r, --recursive               Read files under each directory, recursively
      -R, --dereference-recursive   Dereference symlinks while doing so

    Other Options:
      --line-buffered           Disable output buffering when writing to non-tty

    Exit status:
      0 if match found
      1 if no match found
      2 if error

    Example invocations:
      # Look for a few IPs in all networks found in /etc.
      ipgrep -C 5 -a net -a oldnet -r 192.168.2.5,192.168.2.78 /etc/*

      # Output linefeed separated IPs of all IPv4 hosts/interfaces.
      ipgrep -m within -o 0.0.0.0/0 input.txt

It's slightly more readable/concise than the Rust clap output.
See ``ipgrep --help`` for the actual output, which should be 100% compatible.


-------
License
-------

GPLv3+


-----------------
Things left to do
-----------------

- Maybe make colored output option. Right now you can always disable it
  by piping to ``cat``.
- There are a few *TODOs* in the source to tackle. Not a big priority.


.. |EXAMPLE| image:: assets/example.png
    :alt: CLI output of ipcalc piped to ipgrep showing IP CIDR match with color
