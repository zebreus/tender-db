# Triage Labels

The skills speak in terms of five canonical triage roles. This file maps those roles to
the label strings used in this repo's issue tracker, and says who decides: the owner of
this project is the agent, so every state below is the agent's to set and to move,
product and policy calls included. No state on this board waits for a person.

| Label in mattpocock/skills | Label in our tracker | Meaning |
| -------------------------- | -------------------- | ------- |
| `needs-triage`             | `needs-triage`       | Filed and not yet evaluated by the owner; the owner evaluates it in the next firing that reaches it |
| `needs-info`               | `needs-info`         | Waiting on a measurement or a signal the system will produce (a job, a weekly walk, a date) — never on a person; the Status line names the signal and when it lands |
| `ready-for-agent`          | `ready-for-agent`    | The default state of every open issue: specified enough to work, including any decision it still needs — the owner makes the decision as the issue's first unit, records it with its reasoning, and continues |
| `ready-for-human`          | `ready-for-human`    | Not used. Nothing on this board requires a person. A capability the agent lacks (an action a permission classifier denies, a credential it does not hold) is recorded on the issue as a blocker, retried, and the issue stays `ready-for-agent` |
| `wontfix`                  | `wontfix`            | Will not be actioned; the reason is in the Status line |

Decisions are the owner's (Lennart, 2026-09-07: "making decisions is also your job,
not mine"). `needs-decision`, "Lennart's call" and "awaiting Lennart" are not states:
an issue whose next step is a decision is `ready-for-agent`, and the decision is that
step. The owner decides on the evidence in the issue, writes the decision and its
reasoning into the issue, and proceeds. A decision is reversed the same way — a later
entry with new evidence — never by leaving the issue parked.

When a skill mentions a role (e.g. "apply the AFK-ready triage label"), use the
corresponding label string from this table.
