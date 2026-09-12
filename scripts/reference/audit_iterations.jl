using NetworkLayout

adjacency = [0.0 1.0 0.0; 1.0 0.0 1.0; 0.0 1.0 0.0]
initial = [[-1.0, 0.0], [0.2, 0.8], [1.0, -0.3]]
algorithm = Spring(; initialpos=initial, iterations=5)
items = collect(Iterators.Stateful(LayoutIterator(algorithm, adjacency)))
callable = algorithm(adjacency)

@assert length(items) == 5
@assert callable == items[end - 1]
@assert callable != items[end]
println("iterator: 5 items = initial position + 4 updates")
println("layout: returned item 4 = initial position + 3 updates")
