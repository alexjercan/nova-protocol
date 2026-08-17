# Notes

- The arena's PostUpdate ownership guard only asserted the frozen state. It did not reconcile clocks back to the active match state after NOVA OS closed.
- The guard now derives clock state from the result flow and the complete PauseStates freeze axis.
- Clock resume is limited to an active match. It does not take ownership in the lobby.
