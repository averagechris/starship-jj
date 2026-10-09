# Upstream review

Reviewed on 2026-10-09 against
[lanastara_foss/starship-jj](https://gitlab.com/lanastara_foss/starship-jj).
The upstream `main` cutoff is `74c94705bad6f0f8019db6a4eb093611818dd94a`.
All three commits after the previous cutoff, `8ca6a957`, were reviewed.

| Upstream commit | Decision | Reason |
| --- | --- | --- |
| `4e3017d3d2d6e4b4c69a9a3c25461004ca587775` | Keep fork pins | The change updates fenix, nixpkgs, and rust-analyzer pins. The fork has independently maintained packaging and release inputs; replacing its lockfile provides no behavioral fix. |
| `ad6465088b1f089759e15c61708917449f83d615` | Already superseded | Upstream migrates to jj 0.43. The fork already uses jj-lib and jj-cli 0.44 and its own lazy state and prompt implementation. Porting the older API migration would undo that work. |
| `74c94705bad6f0f8019db6a4eb093611818dd94a` | Not needed by current fork | Upstream raises the Rust macro recursion limit to 256 after its API migration. The fork's test binaries compile without a recursion-limit error, so no compiler setting was added. |

The code and dependencies remain unchanged. Future reviews start after the
full cutoff above and preserve the prompt performance decisions described
in `README.md`.
