# Connector regression report

Recorded on **2026-10-30** from the user's supplied regression summary. These are user-reported results through the actual connector, not a new automated run or an independently inspected transcript. The user linked the full results in [Pensieve](https://pensieve.talesfrom.earth/#root/mcp5fadac97f30d6ee11fa97bf1).

| Scenario | Reported result |
| --- | --- |
| Timeout cancellation | Fixed in the reproduced command scenario. |
| Binary file round-trip | Passed. |
| Permission restrictions | Passed. |
| Failed-command reporting | Passed. |
| Output limits | Passed. |
| Auditing | Passed. |
| Session closure | Passed. |
| Concurrent execution | Still sequential through the connector. |

The user reported that all test artifacts were cleaned up and no application or deployment configuration changed. The supplied summary does not identify the tested binary hash, exact commands or termination evidence fields. It therefore closes the reproduced connector timeout scenario without attesting every timeout or cancellation path.

## Remaining acceptance

- Test child processes and cancellation races through the connector, including timeout near normal completion and transport loss during dispatch/observation. Verify side effects and audit evidence; uncertain outcomes must remain unknown.
- Existing [native/Docker supervision evidence](../deploy/gont/supervision-verification.json) covers TERM-ignoring children, detached process groups, background descendants and transport loss. That evidence is separate from this connector report and does not complete connector cancellation-race acceptance.
- The [local native benchmark](local-verification.md) now covers three sequential/parallel pairs, remote overlap and audit correlation across separate sessions. Compare it with the public adapter/connector path; a single session intentionally permits one active command.
- Compare native results with the public adapter/connector path to locate serialization. Prior recorded native testing completed two 3-second commands in 3.54 seconds; exact-runtime adapter concurrency also passed. Those checks demonstrate overlap in the tested conditions, but do not establish where current connector serialization occurs.

The timeout fix is confirmed for the reproduced scenario. Broader child-process and cancellation-race acceptance remains open before relying on unattended destructive commands. See the [roadmap](roadmap.md) for operational recovery and other remaining work.
