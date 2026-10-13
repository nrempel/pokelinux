# Installed native Messages joint fixture

This isolated root entry forwards the actual `pokelinux-session::Observer` into
`run_with_messages_and_session`, with the actual Messages storage worker. Its
launcher owns the original Hyprland child pidfd and passes the observer the matching
handle and credentialed seqpacket. The actor launches the real installed Qt app as
UID11010. The source is the staged matching Hyprland0.56.2-4 plugin, including the
actual focused socket's SCM_RIGHTS pidfd; there is no synthetic facts producer.

The pre-composition `red-before-composition-3` selection ran before this lane's
Rust entrypoint existed. It executed the previously staged normal service and
actual diagnostic Qt renderer. Unknown initially denied; manual root controls
could expose local text without a source; original focus plus a separate grant
could not reach the app without those controls. Both desired assertions failed.
A later replay is regression evidence, not a new chronological RED.

The diagnostic `pokelinux-messages-observer` ELF links the unchanged production
MessagesClient, view and theme components. It observes body/model state and actual
Qt image readback. `--production` selects the separate normal Qt ELF with its six
standard arguments and uses original compositor screencopy for visual inspection.
Screenshot byte changes alone do not establish displayed text: inspect the
unknown, granted, revoked, restored and unloaded frames before reporting them.

Build and execute on Harbour using the already provisioned private stages:

```sh
python3 tools/native-session-installed-app/build.py --label new-build-label
python3 tools/native-session-installed-app/run.py --phase green --label new-controls-label --build new-build-label --vm-key controls --loss unload --skip-lock
python3 tools/native-session-installed-app/run.py --phase green --label new-production-label --build new-build-label --vm-key production --production
```

Every build freezes selected production files before compilation; every run
freezes driver/binary inputs. Existing labels are never overwritten. Each VM key
has its own clone and lock. All QEMU jobs have no NIC, read-only backing mounts,
`snapshot=on`, four guest vCPUs, and a 6 CPU/8 GiB container quota. Builds allow 8 CPU/
12 GiB. Reuse existing clones when disk is constrained; no unrelated workloads,
Docker images, physical phone, credentials or private artifacts may be changed.

Prepare graphics before the first adapter load. The original 100ms CLOCK_BOOTTIME
sample lease is unchanged. Failed sources remain terminal; source revival and
lease extension are forbidden. `--loss stopped` and `--loss death` select separate
source epochs. Current procfs and common PID/user/mount namespaces are deliberately
qualified within this isolated guest, not assumed for production provisioning.

Qt grabWindow is suppressed by the actual lock. The diagnostic therefore checks
real locked source/actor state and responsive held storage, then observes clearing
after explicit fixture unlock while a separate grant is denied. Released UI bytes,
queued Qt callbacks, secure lock presentation, PAM/restoration containment,
production source/configuration installation and Pixel hardware remain gates.
The explicit force-unlock command is test infrastructure, not a production unlock.

The full lock availability selection currently fails after actual locked graphics
exceeds the original sample lease, including an attempt prepared before the first
source frame. Keep that failure as an availability gate. `--skip-lock` executes
the passing independent source-loss epochs; it does not qualify restoration.

The selected library snapshots predate the separate receiving/read-owner split.
That change introduces `--messages-read-worker`; a combined fixture dispatch and
fresh original-compositor/normal-app replay are required when the branch is merged.
The selected runs here do not qualify the future combined receiver boundary.

`fd-ownership-test.py` freezes the selected pre-change shared core and checks the
actual host-kernel numeric `3`/`03` descriptor alias. It is fixture ownership evidence,
separate from the joint ARM application boundary. The guarded entry is built with
`build.py --from-build current-joint-build-2` to keep the selected core unchanged.
