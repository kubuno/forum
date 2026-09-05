# Changelog

All notable changes to **kubuno-forum** are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this
project adheres to [Semantic Versioning](https://semver.org/). Entries are added under
`[Unreleased]` **as the change is made**; `_tools/release.sh` stamps them under the version
number at release time, and CI publishes that section as the GitHub Release notes.

## [Unreleased]

### Added

- **Watching a topic or forum now actually notifies you.** Subscribing already
  worked, but the subscriptions did nothing; a reply to a watched topic, or a new
  topic in a watched forum, now sends the watcher a notification — while never
  notifying anyone who cannot see the forum.
- **Moderation keeps the people involved informed.** Reporting a post now alerts
  the forum's moderators; resolving a report tells the reporter; approving or
  rejecting a held message, and warning or banning a member, now notify the
  member concerned (a warning carries its reason; a ban does not disclose the
  moderators' private note).
- **`@mention` autocomplete in the reply box.** Typing "@" opens a keyboard-
  navigable search of members; the people you pick are notified of your reply
  (subject, as always, to being able to see the forum).
- **Notifications from a busy topic are grouped.** Several replies (or reactions)
  to the same topic now fold into a single "X and 4 others replied" entry instead
  of a separate notification each, and native (mobile) pushes now carry a title
  and a short description instead of arriving blank.
- **Missed notifications are caught up on your next visit.** The forum's own bell
  is gone — everything goes through the core's shared header bell — but until now
  a notification created while you were offline only reached you through the
  real-time bridge, so it was silently lost if you weren't connected at the time.
  Opening the forum now announces any notification you haven't read yet into the
  shared bell, using the same identity the real-time bridge uses so nothing is
  ever announced twice.
- **Syntax highlighting for code blocks.** Fenced code blocks (```rust, ```js …)
  are now colourised by language; inline code and everything else renders exactly
  as before.
- **Predefined report reasons.** Reporting a post now offers a set of reasons an
  administrator curates (Spam, Inappropriate, Off-topic, Harassment…) as one-tap
  chips, plus an optional comment, instead of only a free-text box.
- **Quoting a post attributes its author.** The quote action now prefixes the
  quoted text with "@Name wrote:" instead of a bare quote.
- **Shareable permalinks for messages.** Each message has its own link (a "Copy
  link" action in its menu); opening one scrolls to the message and briefly
  highlights it.
- **A trash for topics.** A deleted topic now lands in a moderation "Trash" tab
  where a moderator can restore it; an administrator can also delete it for good.
- **Write/Preview in the composer.** The message editor now has a preview tab
  that renders your Markdown exactly as it will appear.
- **Post edit history.** A message's author and the moderators can now open its
  edit history to read every previous version (new `forum.post_revisions`).
- **Advanced search filters.** Search gains an advanced panel — by author, by
  forum, title/body scope, sort and time period — always kept in sync with the
  plain search bar.
- **Prune inactive topics.** A forum admin can move to the trash every topic in a
  forum with no activity for N days (with a preview count first); pinned, solved
  and poll topics are spared.
- **Bulk topic moderation.** A moderator can select several topics from a forum
  and lock, unlock or delete them in one action; rights are re-checked per topic.
- **"Who's online" and a leaderboard.** New community pages — a detailed presence
  list and a top-contributors ranking — linked from the sidebar; a presence
  location never reveals a topic in a forum you cannot see.
- **Word censor.** An administrator can maintain a list of censored words that are
  substituted in displayed messages; the replacement is applied on the server, so
  it cannot be bypassed from the browser.
- **Per-group forum permissions.** In a forum's permissions window, an
  administrator can now grant view/post/reply/attach access to specific instance
  groups — opening an otherwise-restricted forum to just those groups. Group
  grants are purely additive on top of the role-based rules: they can only widen
  access, never remove it.
- **IP and email bans.** In addition to banning an account, a moderator can now
  ban an IP address or an email address (exact match) from the Bans tab. Bans are
  enforced on every authenticated request — a banned visitor is locked out, not
  just prevented from posting — and an administrator is never locked out by one.
- **Moderation team page.** A new "Moderation team" page (`/forum/team`) lists the
  moderators of each forum you can see, grouped by forum.
- **Topic view polish.** Hover a post's reactions to see who reacted (resolved on
  demand), jump straight to the first unread post, hop to any forum from a
  jumpbox in the header, and print a clean copy of the thread.
- **Editable FAQ.** An administrator curates a list of question/answer pairs
  (Markdown answers) from the console; every member reads them on a dedicated
  `/forum/faq` page.
- **Personal RSS/Atom feeds.** Mint a private feed link from the new "RSS feeds"
  page (`/forum/feeds`) and follow recent topics in any feed reader; a feed shows
  exactly what you can see, and revoking a link disables it at once.
- **Custom profile fields.** An administrator can define profile fields (short
  text, long text, yes/no, link, date, or drop-down); every member fills in their
  own answers, shown on their profile page.
- **Ignore list ("foes").** Ignore a member from the topic view or their profile
  menu; their posts then fold under a dismissible banner you can expand at will,
  and a new "Ignored members" page (`/forum/ignored`) lets you review and lift
  each one. The list is yours alone and never affects what others see.
- **Private messages.** Start a one-to-one or small-group conversation (up to ten
  recipients), reply, mark a thread as read, remove it from your own inbox, and
  block a member from messaging you. An inbox lives at `/forum/pm` with a
  conversation view, a compose window, and an unread badge in the sidebar. The
  threads are backed by dedicated tables, kept entirely separate from the boards
  and never indexed by search. A new private message notifies the other
  participants through the shared notification bell — never the message content,
  only who wrote to you.

### Security

- **RSS/Atom feeds never leak restricted content.** A feed is reachable only
  through an unguessable, revocable token in its URL — there is no anonymous,
  token-less feed — and it lists only the topics its owner may see, evaluated as
  an ordinary member (never with administrator visibility). Titles and names are
  XML-escaped.
- **Custom profile field values can't smuggle markup or scripts.** URLs are
  restricted to `http(s)` and validated on the server; every field value is
  rendered as plain, escaped text — never Markdown or HTML.
- **Search no longer reveals restricted forums.** A search could return the
  title and an excerpt of any post, including from forums a member is not
  allowed to read. Results are now limited to the forums the searcher may see,
  the same rule the topic lists already follow, and never include messages that
  were removed or are still awaiting approval.
- **Removed and pending messages are no longer readable by direct link.** A
  message hidden by a moderator, or one still waiting for approval, could be
  opened through a link to its id even though it was gone from the topic. It is
  now visible only to its author and to moderators, and such a message can no
  longer be edited back into place.
- **Voting in a poll now obeys the topic's rules.** A poll vote did not check
  anything: anyone could vote in a poll belonging to a restricted, locked or
  unapproved topic. A vote now requires the same access as posting in that
  topic.
- **Restricted forums and their categories are hidden from the lists.** The
  forum and category listings returned every forum regardless of its view
  permission; a category made only of restricted forums is likewise no longer
  listed to members who cannot see its contents.
- **Sturdier against abusive requests.** Request bodies are now capped in size,
  and the lists a single request can carry — mentions, tags, poll options and
  votes — are bounded, so one call can no longer be turned into thousands of
  database writes. Poll options are validated (2–20 per poll, length-limited)
  instead of surfacing a raw database error.
- **Tighter checks on subscribing and on the moderator roster.** Watching a
  topic or forum, marking it read, and reading a forum's moderator list now all
  require permission to see that forum.
- **Moderators only see their own forums' reports and history.** The report
  queue and the moderation log returned every forum's entries to any moderator;
  they are now limited to the forums a moderator actually manages (plus, for the
  log, that moderator's own actions), while administrators still see everything.
- **Moving, splitting and merging topics now check the other forum too.** These
  actions only verified the forum a topic came from, letting a moderator push a
  topic into — or pull one out of — a forum they do not manage. Both sides are
  now checked, and the destination forum must actually exist.
- **A fuller moderation audit trail.** Locking, unlocking, moving, splitting,
  merging and deleting a topic, marking a solution and resolving a report are now
  all recorded in the moderation log, which previously only captured warnings,
  bans and post approvals.
- **A post can no longer be reported twice.** Reporting the same message again
  while a report is still open is now a no-op instead of piling duplicate
  entries onto the moderators' queue. (The guard was keyed on a report status
  the database never actually used, so it had been inert; it now matches the
  real open state.)
- **Mentions cannot reveal a hidden topic.** Mentioning someone in a restricted
  forum no longer notifies — or discloses the topic's existence to — a member
  who could not otherwise see it.
- **Attachment permission is enforced.** A forum that forbids attachments now
  actually blocks them, and a single message can carry a bounded number of
  attachments.
- **A ban now covers more than posting.** A banned member can no longer react,
  report or vote in polls, not just be blocked from starting posts and topics.
- **Write requests are rate-limited per member.** Every state-changing action
  (posting, reacting, reporting, voting, editing…) now shares a per-member
  budget, so a single account — human or scripted — can no longer hammer the
  board with writes; exceeding it returns a "too many requests" response with a
  retry delay. Reading is never limited.
- **Links and images only allow safe address schemes.** A link or image in a
  message whose address uses a dangerous scheme (such as `javascript:` or
  `data:`) is now dropped rather than rendered, closing a cross-site-scripting
  avenue; ordinary `http(s)`, `mailto:` and `tel:` links are unaffected.
- **You can no longer ban yourself.** Banning your own account is refused, so a
  moderator cannot accidentally lock themselves out.
- **Structural administration is now audited.** Creating or deleting a forum,
  changing a forum's permissions and adding or removing a moderator are recorded
  in the moderation log alongside the content-moderation actions.
- **Deleting a topic no longer destroys it irreversibly.** A deleted topic (by
  its author or a moderator) is now hidden from every listing but kept and
  recoverable, matching how removed posts already work, instead of being erased
  along with all its replies. Counts and "last post" markers correctly ignore
  hidden topics and posts.

### Changed


- **The README now opens with the module's logo.** The public README on
  GitHub now shows the module's designer logo (the same PNG shown as the
  browser tab icon and in the applications menu) at the top of the page — the
  repository landing now matches the icon a signed-in user sees inside the
  platform. The image ships in-repo, under `.github/logo.png`, so it renders
  even when the repo is browsed offline.

- **Forum notifications now use the platform's shared bell.** Instead of a second
  bell inside the forum, replies, mentions, moderation actions and the rest are
  delivered in real time to the single notification bell in the top bar that every
  module shares — one place for everything, no forum-only inbox.
- **Notifications name who acted.** A notification now reads "Alice replied"
  rather than a generic "New reply" whenever the actor's name can be resolved.
- **Admin panel edits in place.** Adding or renaming a category, rank or report
  reason now happens through an inline row (Save/Cancel) instead of a modal
  prompt, matching how a post is edited in a topic.
- **Search now runs on a real full-text index.** Post and topic search uses
  PostgreSQL full-text search (relevance ranking, highlighted excerpts, phrase
  and exclusion operators) instead of a slow substring scan.

- **Long forums and topics are now paginated.** Topic lists and topic pages
  previously showed only their first batch and silently hid the rest — a busy
  forum's later topics, and any discussion past its hundredth message, were
  unreachable. A pager now walks through every page, and posting a reply jumps
  to the last page so the new message is in view.
- **A navigation trail on forums and topics.** A breadcrumb (Category › Forum ›
  Sub-forum › Topic) now sits at the top of every forum and topic, so you always
  know where you are and can jump back up a level in one click.
- **Author ranks and post counts appear beside each message.** The column next
  to a message now shows the author's rank (earned from their post count, or an
  assigned special rank) and how many messages they have written.
- **Signatures are shown again.** A member's signature is now displayed under
  their messages, honouring both the board-wide "signatures allowed" setting and
  each reader's own "show signatures" preference — until now it was stored but
  never rendered.
- **Member profiles you can actually open.** Every member now has a profile page
  — avatar, rank, join date, post and topic counts, likes, an "about" section and
  their recent topics — reachable by clicking an author beside any message.
- **A members directory.** A new Members page lists everyone who takes part,
  sortable by most posts, newest, or recently active, and paginated; it is linked
  from the forum sidebar.
- **The moderation panel now has tabs — and shows the log and the bans.** Reports
  and the approval queue are joined by two views that had no interface at all
  before: the moderation log (who did what, and when) and the list of banned
  members, each ban liftable in one click.
- **You can attach a poll when starting a topic.** The new-topic composer now has
  a poll builder — a question, two to twenty options, single or multiple choice,
  and an optional run time — so polls can finally be created from the interface.
- **A "Drafts" page.** Your saved new-topic and reply drafts are now listed on
  their own page, each resumable in one click or removable.
- **"Mark read" and "Mark all read."** A forum's topic list gains a "mark read"
  action and the forum index a "mark all read", clearing unread state across
  every visible topic at once.
- **Search and the cross-forum feed are paginated.** Search results and the
  Recent / Unanswered / Popular / Unread / Mine feeds now page through their
  results instead of loading only the first batch.
- **Richer profile settings.** You can now edit your bio, location, website and
  a custom title from the forum settings, not just your signature.
- **Drafts are saved as you type.** Composing a new topic or a reply now
  autosaves a draft in the background every few seconds, and clears it once the
  message is posted.
- **A proper forum edit form in the admin console.** Editing a forum now offers
  every field — name, description, colour, icon, rules, parent forum, position,
  locked and read-only — instead of a single rename prompt, and forums can be
  moved up or down within their category.
- **Admins can assign a special rank.** A member can be given (or cleared of) a
  special rank directly from the ranks administration.
- **New Forum logo** — a blue hexagon with three people and speech bubbles,
  used as the browser-tab icon and in the applications menu. It replaces the
  generic conversation icon and is raster (PNG) designer artwork; the Forum
  tab now has an icon of its own.




### Fixed


- **A withdrawn dependency is no longer used.** A crate deep in the tree
  (`spin` 0.9.8, pulled in through the HTTP stack) was yanked by its authors.
  No vulnerability was announced, but a withdrawn crate has no business in a
  release; the lockfile now takes the version that replaced it.
- **The package could not be built where `zip` is absent.** The Windows job of
  the continuous integration has no `zip`, so the Windows package was simply lost
  the first time it was attempted — a script failure, not a build failure. The
  builder now falls back to 7-Zip, then to PowerShell.
### Added

- **This module now ships a `.kbpkg`** — the single package format a Kubuno
  server installs by itself, the same file on Linux, Windows and macOS. It
  carries the same binary, interface and manifest as the system packages,
  arranged the way the server expects to find a module on disk, plus a
  `SHA256SUMS` so a copy carried offline can be checked without the catalogue.
  Nothing changes for existing installations: the `.deb`, `.rpm`, `.exe` and
  `.pkg` are still published, and a catalogue that sees both simply prefers the
  new one. It is also the only format the server can unpack without an external
  tool, which is what makes one-click installation possible away from
  Debian-like systems.
### Fixed

- **A built package could be thrown away instead of published.** The job that
  attaches a package to the release waited ten minutes for another workflow to
  create that release, then gave up with "release never appeared — build.yml
  likely failed". The diagnosis was wrong: on a repository whose `.deb` takes
  longer than ten minutes to build, the release simply did not exist yet, and a
  package that had built perfectly was discarded. Four modules reached v0.1.6
  with packages missing for some systems because of it. The job now creates the
  release itself when it is missing, so it no longer depends on another workflow
  finishing first.
### Added

- **Security policy and CI quality gate.** A `SECURITY.md` documents how to
  report vulnerabilities, and a CI workflow enforces `clippy -D warnings`, a
  dependency-vulnerability audit (`cargo audit`) and the frontend typecheck/tests.

### Security

- **Forum now authenticates proxied requests from a signed token instead of
  trusting plain headers.** Requests must carry a valid `X-Kubuno-Auth` token
  minted by the core with this module's internal secret (see `kubuno-modauth`),
  rather than reading `X-Kubuno-User-*` headers at face value — which any process
  reaching Forum's loopback port could otherwise forge to act as any user.

## [0.1.6] - 2026-08-19

### Changed

- Theme tokens: two colours for navigation labels (`--color-text-nav`,
  `--color-text-nav-active`). Every module carries the same token sheet, so the
  values must match across them — whichever bundle loads last would otherwise
  win. No visible change inside this module.

### Changed

- Default application background token aligned with the core (`--body-bg` `#f8fafd`). Only
  visible when the module runs standalone: inside the shell the active theme sets it.

[Unreleased]: https://github.com/kubuno/forum/compare/v0.1.6...HEAD
[0.1.6]: https://github.com/kubuno/forum/releases/tag/v0.1.6
