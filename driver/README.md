# Driver Board Firmware

## Axes

| Joint     | Function                 | G-code Letter | Units | Range | Encoder Counts | Per Count  |
| --------- | ------------------------ | ------------- | ----- | ----- | -------------- | ---------- |
| PL1       | Linear vertical slideway | Z             | mm    | 0-940 | 3554           | 0.2267 mm  |
| PL5       | Shoulder                 | A             | deg   | 180°  | 5260           | 0.03422 mm |
| PL6       | Elbow                    | B             | deg   | 331°  | 4836           | 0.06844 mm |
| PL4       | Wrist yaw                | C             | deg   | 220   | 2142           | 0.10267 mm |
| PL2 + PL3 | Wrist Pitch (w1 + w2)    | U             | deg   | n/a   | 2750           | 0.07415 mm |
| PL2 + PL3 | Wrist Roll (w1 - w2)     | V             | deg   | n/a   | 8442           | 0.07415 mm |
| PL7       | Gripper jaw              | W             | mm    | 0-90  | 1200           |            |

## References

- [Inside RTX](https://wiki.london.hackspace.org.uk/w/images/3/3c/RTX_Inside.pdf)
