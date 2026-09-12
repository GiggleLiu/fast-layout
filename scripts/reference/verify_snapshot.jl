using SHA
using TOML

root = normpath(joinpath(@__DIR__, "../../upstream/NetworkLayout.jl"))
snapshot = TOML.parsefile(joinpath(root, "SNAPSHOT.toml"))
for (path, expected) in snapshot["sha256"]
    actual = bytes2hex(open(sha256, joinpath(root, path)))
    actual == expected || error("$path: expected $expected, got $actual")
end
println("verified $(length(snapshot["sha256"])) files at $(snapshot["commit"])")
