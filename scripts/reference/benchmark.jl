using NetworkLayout
using Printf
using SparseArrays

function graph(n, dim)
    edges = Set((min(i, (i - 1 + offset) % n + 1), max(i, (i - 1 + offset) % n + 1))
                for i in 1:n for offset in (1, 7, 31))
    rows = [x for (u, v) in edges for x in (u, v)]
    cols = [x for (u, v) in edges for x in (v, u)]
    adjacency = sparse(rows, cols, ones(length(rows)), n, n)
    initial = dim == 2 ? [[sin(i), cos(i)] for i in 1:n] :
                         [[sin(i), cos(i), sin(2i)] for i in 1:n]
    adjacency, initial
end

function timed(f; warmups=1, samples=3)
    for _ in 1:warmups
        f()
    end
    sort!([@elapsed f() for _ in 1:samples])
end

consume(iterator) = foreach(_ -> nothing, iterator)

println("# Julia reference baseline")
println()
println("Julia: `$(VERSION)`")
println("Threads: `$(Threads.nthreads())`")
println("CPU: `$(Sys.cpu_info()[1].model)`")
println("Warmups: 1; samples: 3; reported value: median seconds")
println("Dimensions: 2 and 3; circulant graph: n vertices, offsets 1, 7, and 31, 3n undirected edges")
println("Tolerance: disabled; 2D initial coordinates: `(sin(i), cos(i))`; 3D adds `sin(2i)`")
println()
println("| algorithm | nodes | dim | updates | median seconds |")
println("| --- | ---: | ---: | ---: | ---: |")
for n in (100, 500, 1000), dim in (2, 3)
    adjacency, initial = graph(n, dim)
    spring_updates = 20
    spring = Spring(; initialpos=initial, iterations=spring_updates + 1)
    spring_times = timed(() -> consume(LayoutIterator(spring, adjacency)))
    @printf("| spring | %d | %d | %d | %.6f |\n", n, dim, spring_updates, spring_times[2])

    # Exact stress allocates dense n by n matrices and computes a pseudoinverse.
    # Five updates keeps the 1000-node baseline practical while exposing that ceiling.
    stress_updates = 5
    stress_layout = Stress(; initialpos=initial, iterations=stress_updates + 1,
        abstols=0.0, reltols=0.0, abstolx=0.0)
    stress_times = timed(() -> consume(LayoutIterator(stress_layout, adjacency)))
    @printf("| stress | %d | %d | %d | %.6f |\n", n, dim, stress_updates, stress_times[2])
end
