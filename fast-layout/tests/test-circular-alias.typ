#import "@preview/fast-layout:0.1.0": layout

#let shell = layout(4, (), algorithm: "shell")
#let circular = layout(4, (), algorithm: "circular")
#assert.eq(circular.positions, shell.positions)
The circular alias maps to shell layout.
