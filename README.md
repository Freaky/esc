# ESC: Email Sucks Completely

## (Or: Email Search Command)

This is decidedly *not* an Email Search Command.  It's more a proof-of-concept
toy I'm hacking together because I want to play with [Tantivy].

It can index my ~2 million-strong `~/Maildir` in under three minutes on my 12
core Westmere Xeon with a couple of SSDs, and then find the top 4 [FreshBSD]
exception emails in about 30 milliseconds.

If you want a proper email search tool, I recommend [notmuch].  I hacked most of
this together while waiting for `notmuch new` to catch up a few tens of
thousands of messages.

The name is a homage to [HSC], the static website generator I used in the late
1990's.

### Synopsis

```
-% mkdir /tmp/email_sucks_completely
-% esc index ~/Maildir
[10000 23080.88/sec] 433.259025ms
[20000 25133.71/sec] 795.74395ms
...
[2020000 14512.78/sec] 139.187685983s
[2030000 14536.14/sec] 139.651889859s
Indexed 2032748 messages in 143.837522255s
Final merge finished after 166.397419871s
-%
-% esc search "notmuch mail"
/home/freaky/Maildir/.archive.2016.lists.notmuch/cur/1452984894.98194_0.voi.aagh.net:2,: Mail merge with notmuch
/home/freaky/Maildir/.archive.2012.lists.freebsd-ports-bugs/cur/1330172492.25158_0.voi.nightsdawn.sf:2,: ports/165468: New port: mail/notmuch
/home/freaky/Maildir/.archive.2016.lists.notmuch/cur/1453365489.20630_0.voi.aagh.net:2,: Re: Mail merge with notmuch
...
searched in 40.005954ms
```

[Tantivy]: https://github.com/tantivy-search/tantivy
[FreshBSD]: https://v4.freshbsd.org
[notmuch]: https://notmuchmail.org
[HSC]: https://github.com/mbethke/hsc
