# ESC: Email Search Command

## (Or: Email Sucks Completely)

ESC is a simple proof-of-concept toy I'm hacking together because I want to
play with [Tantivy] and would quite like a fast command-line email search tool.

It can index my ~2 million-strong `~/Maildir` in under three minutes on my 12
core Westmere Xeon with a couple of SSDs, and answer most queries in under 40ms.

If you want a *useful* email search tool, I recommend [notmuch].  I hacked most
of this together while waiting for `notmuch new` to catch up a few tens of
thousands of messages, and it needs a lot of love to be more than a vague
curiosity.

The name is a homage to [HSC], the static website generator I used in the late
1990's.  Because why not.  Email does suck.

### Synopsis

Indexing is multithreaded: one walks your maildirs, `read-threads` threads read
and parse the emails, and `index-threads` write them to the search index.  You
should adjust them to taste based on available IO and CPU and how much data you
have to index.

If no index directory is specified, it defaults to `/tmp/email-sucks-completely`.

```
-% esc -d ~/.local/esc index --read-threads=8 --index-threads=4 ~/Maildir
[10000 23080.88/sec] 433.259025ms
[20000 25133.71/sec] 795.74395ms
...
[2020000 14512.78/sec] 139.187685983s
[2030000 14536.14/sec] 139.651889859s
Indexed 2032748 messages in 143.837522255s
Final merge finished after 166.397419871s
-%
-% esc -d ~/.local/esc search "notmuch mail"
/home/freaky/Maildir/.archive.2016.lists.notmuch/cur/1452984894.98194_0.voi.aagh.net:2,: Mail merge with notmuch
/home/freaky/Maildir/.archive.2012.lists.freebsd-ports-bugs/cur/1330172492.25158_0.voi.nightsdawn.sf:2,: ports/165468: New port: mail/notmuch
/home/freaky/Maildir/.archive.2016.lists.notmuch/cur/1453365489.20630_0.voi.aagh.net:2,: Re: Mail merge with notmuch
...
searched in 40.005954ms
```

[Tantivy]: https://github.com/tantivy-search/tantivy
[notmuch]: https://notmuchmail.org
[HSC]: https://github.com/mbethke/hsc
