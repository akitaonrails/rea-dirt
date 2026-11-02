# Contributions back

Fixes found while building this launcher that the community lacks, drafted
for PCGamingWiki, upstream projects, ProtonDB or the WineHQ AppDB
(`sdk/docs/playbooks/contribute-back.md`). These are public posts under the
owner's name: agents only draft them here; the owner reviews and posts.
Only fixes verified in the real game.

| Target (page or project) | Fix | Evidence | Status |
|---|---|---|---|
| PCGamingWiki, Colin McRae: DiRT (multi-core crash) | Avoid the 6-core worker map | `docs/RESEARCH.md` "CPU maps"; a race loads on a 32-thread CPU with the 12-core map (`docs/RUNTIME_NOTES.md`, 2026-11-01). The 6-core case itself is not run-tested yet | draft, needs a 6-thread run first |

## Drafts

### PCGamingWiki: the 6-core worker map

Applies to the official v1.22 `DiRT.exe` (SHA-256
`e10268f8233b90a9c162e097ffa81b272d49824487ec80ca6d0ba6fd3cb4e3c3`, or
`e4b66bd39e1a516674e885af7d459751bf87bd85e4f69c17e4ab65f67755aaed` with
Large Address Aware set).

> The game's `system/workerMap6Core.xml` (and the copy some multi-core
> fixes ship) puts `core_6` and `core_7` in the `vehicleSingleThreaded`
> group, but only declares workers `core_0` to `core_5`. On a CPU with 6
> logical threads, use the 4-core map instead: in
> `system/hardware_settings_restrictions.xml`, make the `cores int_eq 6`
> rule use `workerMapFile="system/workerMap4Core.xml"` with the 4-core
> processor numbers. Source: https://github.com/akitaonrails/rea-dirt

Hold until a 6-thread machine (or an affinity-limited run) confirms the
crash and the fix.
