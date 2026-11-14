# r-dlp 0.2.2 - bot-checked videos play again

- Videos behind YouTube's bot check resolve again. Since 0.1.1 the anonymous visitor bootstrap failed over HTTP/2 because the watch page sends more headers than the default limit allows.
- The visitor bootstrap reads a 3 KB file instead of the 1.5 MB watch page first: the first bot-checked song resolves in about 0.2 s.
