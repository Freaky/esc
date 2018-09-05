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

[Tantivy]: https://github.com/tantivy-search/tantivy
[FreshBSD]: https://v4.freshbsd.org
[notmuch]: https://notmuchmail.org
[HSC]: https://github.com/mbethke/hsc
