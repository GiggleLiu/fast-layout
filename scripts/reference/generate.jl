using JSON3
using LinearAlgebra
using NetworkLayout
using NetworkLayout: compute_laplacian, make_symmetric, pairwise_distance, stress
using Random

const OUT = joinpath(@__DIR__, "../../fast-layout-engine/tests/fixtures")

matrix(n, edges, weights=ones(length(edges))) = begin
    a = zeros(Float64, n, n)
    for ((u, v), w) in zip(edges, weights)
        a[u + 1, v + 1] = a[v + 1, u + 1] = w
    end
    a
end

rows(points) = [collect(p) for p in points]
distances(points) = [norm(points[i] - points[j]) for i in eachindex(points), j in eachindex(points)]

function updated_positions(algo, adjacency, updates)
    iterator = LayoutIterator(algo, adjacency)
    item, state = iterate(iterator)
    for _ in 1:updates
        next = iterate(iterator, state)
        next === nothing && error("reference stopped before the requested update")
        item, state = next
    end
    item
end

function write_fixture(name; algorithm, nodes, edges, dim=2, iterations=100,
                       initial=Any[], edge_weights=Float64[], node_weights=Float64[],
                       shells=Vector{Int}[], node_sizes=Float64[], root=0,
                       positions, objective=nothing, extra=Dict{String,Any}())
    fixture = Dict(
        "name" => name, "algorithm" => algorithm, "nodes" => nodes,
        "edges" => [collect(e) for e in edges], "dim" => dim,
        "iterations" => iterations, "tolerance" => 0.0, "initial" => initial,
        "edge_weights" => edge_weights, "node_weights" => node_weights,
        "shells" => shells, "node_sizes" => node_sizes, "root" => root,
        "positions" => rows(positions), "pairwise_distances" => distances(positions),
        "objective" => objective,
    )
    merge!(fixture, extra)
    open(joinpath(OUT, "$name.json"), "w") do io
        JSON3.pretty(io, fixture; allow_inf=true)
        println(io)
    end
end

function iterative_fixtures()
    spring_edges = [(0, 1), (1, 2), (2, 3), (3, 4), (4, 0), (0, 2)]
    for dim in (2, 3)
        initial = dim == 2 ? [[-1.0, 0.2], [-0.3, 0.8], [0.4, -0.7], [1.1, 0.1], [0.2, 1.3]] :
                             [[-1.0, 0.2, 0.3], [-0.3, 0.8, -0.4], [0.4, -0.7, 0.9], [1.1, 0.1, -0.8], [0.2, 1.3, 0.5]]
        adjacency = matrix(5, spring_edges)
        updates = 8
        algo = Spring(; initialpos=initial, C=2.0, initialtemp=2.0, iterations=updates + 1)
        positions = updated_positions(algo, adjacency, updates)
        write_fixture("spring_exact_$(dim)d"; algorithm="spring", nodes=5,
            edges=spring_edges, dim, iterations=updates, initial,
            positions, extra=Dict("c" => 2.0, "temperature" => 2.0, "theta" => 0.0))
    end

    cases = [
        ("connected", 5, [(0, 1), (1, 2), (2, 3), (3, 4), (0, 4), (1, 3)], [1.0, 1.7, 0.8, 1.2, 2.1, 0.9]),
        ("disconnected", 6, [(0, 1), (1, 2), (3, 4)], [1.0, 1.5, 0.75]),
    ]
    for (kind, n, edges, weights) in cases, dim in (2, 3)
        initial = [[sin(i + d / 3) + i / 10 for d in 1:dim] for i in 1:n]
        adjacency = matrix(n, edges, weights)
        updates = 6
        algo = Stress(; initialpos=initial, iterations=updates + 1,
            abstols=0.0, reltols=0.0, abstolx=0.0)
        positions = updated_positions(algo, adjacency, updates)
        d = pairwise_distance(adjacency)
        if any(isinf, d)
            finite_max = maximum(filter(isfinite, d))
            components = size(nullspace(NetworkLayout.weightedlaplacian(adjacency)), 2)
            replacement = finite_max * components^(1 / 3)
            replace!(d, Inf => replacement)
        end
        objective = stress(positions, d, d .^ -2)
        write_fixture("stress_weighted_$(kind)_$(dim)d"; algorithm="stress", nodes=n,
            edges, edge_weights=weights, dim, iterations=updates, initial, positions,
            objective, extra=Dict("ideal_distances" => d))
    end
end

function direct_fixtures()
    edges = [(0, 1), (0, 2), (0, 3), (1, 2), (2, 3), (3, 4)]
    adjacency = matrix(5, edges, [1.0, 0.7, 1.2, 0.8, 1.1, 0.9])
    node_weights = [1.0, 2.0, 0.75, 1.5, 1.25]
    symmetric = make_symmetric(adjacency / 2)
    laplacian, degree_matrix = compute_laplacian(symmetric, node_weights)
    normalized = Diagonal(diag(degree_matrix) .^ -0.5) * laplacian * Diagonal(diag(degree_matrix) .^ -0.5)
    values = eigen(Symmetric(normalized)).values
    for dim in (2, 3)
        positions = Spectral(; dim, nodeweights=node_weights)(adjacency / 2)
        write_fixture("spectral_weighted_$(dim)d"; algorithm="spectral", nodes=5,
            edges, edge_weights=[1.0, 0.7, 1.2, 0.8, 1.1, 0.9], dim,
            node_weights, positions,
            extra=Dict("degree" => diag(degree_matrix), "eigenvalues" => values))
    end

    edges5 = [(0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 6),
              (6, 0), (0, 3), (1, 4), (2, 5)]
    weights5 = [1.0, 0.7, 1.3, 0.8, 1.1, 0.9, 1.4, 0.6, 1.2, 0.75]
    node_weights5 = [1.0, 1.5, 0.8, 2.0, 1.1, 0.9, 1.3]
    adjacency5 = matrix(7, edges5, weights5) / 2
    symmetric5 = make_symmetric(adjacency5)
    laplacian5, degree_matrix5 = compute_laplacian(symmetric5, node_weights5)
    normalized5 = Diagonal(diag(degree_matrix5) .^ -0.5) * laplacian5 * Diagonal(diag(degree_matrix5) .^ -0.5)
    values5 = eigen(Symmetric(normalized5)).values
    positions5 = Spectral(; dim=5, nodeweights=node_weights5)(adjacency5)
    write_fixture("spectral_weighted_5d"; algorithm="spectral", nodes=7,
        edges=edges5, edge_weights=weights5, dim=5, node_weights=node_weights5,
        positions=positions5,
        extra=Dict("degree" => diag(degree_matrix5), "eigenvalues" => values5))

    shell_edges = [(i, i + 1) for i in 0:5]
    shells = [[2], [0, 4]]
    shell_positions = Shell(; nlist=[s .+ 1 for s in shells])(matrix(7, shell_edges))
    write_fixture("shell_geometry"; algorithm="shell", nodes=7, edges=shell_edges,
        shells, positions=shell_positions)

    tree_edges = [(0, 1), (0, 2), (0, 3), (1, 4), (1, 5), (2, 6)]
    sizes = [1.0, 2.0, 1.5, 3.0, 0.5, 1.0, 1.0]
    children = [[e[2] + 1 for e in tree_edges if e[1] == i] for i in 0:6]
    tree_positions = Buchheim(; nodesize=sizes)(children)
    write_fixture("buchheim_varying_size"; algorithm="buchheim", nodes=7,
        edges=tree_edges, node_sizes=sizes, positions=tree_positions)
end

function buchheim_random_fixtures()
    rng = MersenneTwister(0x073192ac)
    cases = Any[]
    for case in 1:24
        n = 10 + (case * 17) % 51
        parents = [0, 0, 0, 1, 2, 3]
        append!(parents, [rand(rng, 0:(node - 1)) for node in 7:(n - 1)])
        edges = [(parents[node], node) for node in 1:(n - 1)]
        sizes = [0.25 + 2.75 * rand(rng) for _ in 1:n]
        children = [[e[2] + 1 for e in edges if e[1] == node] for node in 0:(n - 1)]
        positions = Buchheim(; nodesize=sizes)(children)
        push!(cases, Dict(
            "name" => "random_tree_$case", "nodes" => n,
            "edges" => [collect(e) for e in edges], "node_sizes" => sizes,
            "positions" => rows(positions),
        ))
    end
    open(joinpath(OUT, "buchheim_random.json"), "w") do io
        JSON3.pretty(io, cases)
        println(io)
    end
end

mkpath(OUT)
iterative_fixtures()
direct_fixtures()
buchheim_random_fixtures()
