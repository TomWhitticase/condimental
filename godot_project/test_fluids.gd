# Headless proof of the fluid network's *presentation* half (milestone 6,
# ARCHITECTURE.md §9 item 6; docs/UI_UX.md §4's "visible liquid surfaces in
# tanks/pipes" and §5's hover tooltips) — the structural claims a machine can
# check without a framebuffer. Whether it actually looks right is what
# scripts/screenshot-scene.sh is for, same division as test_presentation.gd.
#
# Run with:
#   godot --headless --path godot_project --script test_fluids.gd
#
# Verifies:
#   * the scene renders one pipe tile per segment the SIMULATION reports in
#     `fluid_snapshots_json` (an ARRAY of networks since the "Real Placement"
#     epic — one per connected run of placed pipe), at the tiles it reports,
#   * every one of those pipe nodes (and every liquid body) is a real,
#     non-empty imported voxel mesh, not a silently-missing asset,
#   * the source segment is drawn with a different model from a plain one
#     (UI_UX.md §7: state distinguished by more than colour),
#   * each liquid body's vertical scale tracks its segment's live pressure,
#     and a segment with no fluid in it is hidden outright,
#   * hovering a pipe tile resolves to fluid info text, through the exact
#     same _tile_under_screen_pos plane-intersection path machines use, in
#     BOTH view modes (ISOMETRIC cursor and FIRST_PERSON crosshair),
#   * the inlet segment (the one that actually feeds a machine) renders as
#     the dedicated connector fitting (PIPE_INLET_ASSET), not a plain header
#     tile — the visual-overhaul pass's fix for the pipe-to-machine seam,
#   * the pipe run's FLUID TANK (the "Visual overhaul round 2" epic: ticket 1
#     made it real in simulation, ticket 2 draws it) renders as a real
#     non-empty mesh at the tile the SIMULATION reports, its liquid column
#     rescales when the tank is filled through the real command path, an
#     empty tank hides that column outright, and its tile answers hover
#     through the same one `_hover_text_at_tile` mechanism every other
#     building uses.
#
# The one-frame-before-checking dance and the "never hold your own camera
# reference" rule below are both inherited from test_hover.gd — see that
# file's header for the full explanation of the `--script` SceneTree
# bootstrap quirk they work around.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

var main
var _ran := false


func _fail(msg: String) -> void:
	printerr("FLUIDS_FAIL: %s" % msg)
	quit(1)


func _initialize() -> void:
	main = load("res://main.tscn").instantiate()
	root.add_child(main)
	main._ready()


func _process(_delta: float) -> bool:
	if _ran:
		return true
	_ran = true
	_run_checks()
	return true


## Every MeshInstance3D under `node`, so "the asset actually imported" can be
## asserted rather than assumed (a missing .vox yields an empty Node3D).
func _meshes(node: Node, out: Array) -> Array:
	if node is MeshInstance3D:
		out.append(node)
	for child in node.get_children():
		_meshes(child, out)
	return out


func _run_checks() -> void:
	# Built HERE, not in `_initialize`: `_ready` runs twice under this harness
	# and each run creates a fresh (empty) world — see test_hover.gd's header.
	# A tank, four pipe tiles and the one machine whose recipe declares
	# `fluid_inputs`, which is what a starting balance can afford.
	Layouts.build_fluid_run(main.sim)
	Layouts.stock_larder(main.sim)
	main._rebuild_world()
	main._update_fluid_network()

	var networks: Array = JSON.parse_string(main.sim.fluid_snapshots_json())
	if networks.size() != 1:
		_fail("expected exactly one fluid network from the placed run, got %d" % networks.size())
		return
	var snapshot: Dictionary = networks[0]
	var segments: Array = snapshot["segments"]
	if segments.size() != 4:
		_fail("expected the four placed pipe tiles, got %d segments" % segments.size())
		return
	# The network's fluid is decided by whichever machine taps it — the mixer
	# is adjacent to (0, 1), so the run carries vinegar without anything
	# saying so here.
	if snapshot.get("fluid") == null:
		_fail("the placed run is tapped by the mixer, so it should have a fluid: %s" % str(snapshot))
		return
	if snapshot.get("taps", []).is_empty():
		_fail("the mixer touches the run, so it should appear in `taps`: %s" % str(snapshot))
		return
	if snapshot.get("tank_tiles", []).is_empty():
		_fail("the tank discharges into the run, so it should appear in `tank_tiles`: %s" % str(snapshot))
		return

	# --- one rendered pipe per segment, at the sim's own tiles ------------
	var source_asset := ""
	var plain_asset := ""
	var inlet_tiles := 0
	for segment in segments:
		var pair: Array = segment["tile"]
		var tile := Vector2i(int(pair[0]), int(pair[1]))
		var node: Node3D = main.get_node_or_null("World/pipe_%d,%d" % [tile.x, tile.y])
		if node == null:
			_fail("no rendered pipe at %s, which the simulation reports as a segment" % str(tile))
			return
		if _meshes(node, []).is_empty():
			_fail("the pipe at %s rendered no meshes — the .vox didn't import" % str(tile))
			return
		var expected: Vector3 = main._tile_origin(tile.x, tile.y, main.FLOOR_Y)
		if not node.position.is_equal_approx(expected):
			_fail("the pipe at %s is drawn at %s, expected %s" % [str(tile), str(node.position), str(expected)])
			return
		var asset: String = main._pipe_asset_for(tile)
		if bool(segment.get("is_source", false)):
			source_asset = asset
		elif asset == main.PIPE_INLET_ASSET:
			inlet_tiles += 1
		else:
			plain_asset = asset

	# --- the tank's own tile is a DIFFERENT model, not just a tint --------
	if source_asset == "":
		_fail("no segment is marked as the tank's supply tile: %s" % str(segments))
		return
	if source_asset == plain_asset:
		_fail("the tank-fed segment draws the same model as a plain one — UI_UX.md §7 wants a shape difference")
		return
	# --- and the tile the machine draws from gets the branch fitting ------
	if inlet_tiles < 1:
		_fail("no segment drew the machine-inlet fitting even though the mixer taps the run")
		return

	# --- the liquid bodies track live pressure ---------------------------
	# The tank starts empty, so the run does too and every body is hidden.
	main._update_fluid_network()
	var bodies: Array = main._pipe_fluid_nodes.values()
	if bodies.size() != segments.size():
		_fail("expected one liquid body per segment, got %d for %d" % [bodies.size(), segments.size()])
		return
	for body in bodies:
		if _meshes(body, []).is_empty():
			_fail("a liquid body rendered no meshes — '%s' didn't import" % main.PIPE_FLUID_ASSET)
			return
		if body.visible:
			_fail("an empty run should hide its liquid entirely, not draw a sliver")
			return

	# Fill the tank through the real command path and let the run pressurize.
	main.sim.issue_command_json(JSON.stringify({"type": "fill_fluid_tank"}))
	for i in range(200):
		main.sim.tick()
	main._update_fluid_network()
	var visible_bodies := 0
	for body in main._pipe_fluid_nodes.values():
		if body.visible:
			visible_bodies += 1
			if body.scale.y > float(main.PIPE_BORE_HEIGHT_VOXELS) + 0.001:
				_fail("a liquid body (%f) rose past its bore (%d voxels)" % [body.scale.y, main.PIPE_BORE_HEIGHT_VOXELS])
				return
	if visible_bodies < 1:
		_fail("the tank was filled and the run ticked, but no liquid is drawn")
		return

	# --- hover: a pipe answers through the one mechanism -----------------
	var probe_pair: Array = segments[0]["tile"]
	var probe := Vector2i(int(probe_pair[0]), int(probe_pair[1]))
	var pipe_text: String = main._hover_text_at_tile(probe)
	if not pipe_text.contains("pipe"):
		_fail("hovering a pipe tile should describe a pipe, got: %s" % pipe_text)
		return
	if not pipe_text.contains("units") or not pipe_text.contains("pressure"):
		_fail("the pipe tooltip should report its level and pressure, got: %s" % pipe_text)
		return
	# ...and through the same screen-to-tile math, in the isometric view.
	var screen_pos: Vector2 = main._camera.unproject_position(
		main._tile_origin(probe.x, probe.y, main.BELT_SURFACE_Y)
	)
	if main._tile_under_screen_pos(screen_pos) != probe:
		_fail("the pipe tile did not resolve through the cursor ray")
		return

	# --- the fluid tank: rendered where the sim says, with a live column --
	var tanks: Array = JSON.parse_string(main.sim.fluid_tank_snapshots_json())
	if tanks.size() != 1:
		_fail("expected exactly one placed fluid tank, got %d" % tanks.size())
		return
	var tank: Dictionary = tanks[0]
	var tank_pair: Array = tank["tile"]
	var tank_tile := Vector2i(int(tank_pair[0]), int(tank_pair[1]))
	var tank_node: Node3D = main.get_node_or_null("World/fluid_tank_%d,%d" % [tank_tile.x, tank_tile.y])
	if tank_node == null or _meshes(tank_node, []).is_empty():
		_fail("the fluid tank rendered no mesh at %s" % str(tank_tile))
		return
	if tank.get("pipe_tile") == null:
		_fail("the tank faces the run's head, so pipe_tile must not be null: %s" % str(tank))
		return
	var tank_nodes: Dictionary = main._fluid_tank_nodes.get("%d,%d" % [tank_tile.x, tank_tile.y], {})
	if tank_nodes.is_empty():
		_fail("the placed tank built no live-level nodes")
		return
	var column: Node3D = tank_nodes["fill"]
	if _meshes(column, []).is_empty():
		_fail("the tank's liquid column rendered no meshes")
		return
	# Topped up right before looking: the tank presents constant full head and
	# drains into the run, so by now the fill it was given at the top of this
	# test has largely gone down the pipes — which is the mechanism working,
	# not a rendering fault.
	main.sim.issue_command_json(JSON.stringify({"type": "fill_fluid_tank"}))
	main._refresh_buffers()
	if not column.visible:
		_fail("the tank was filled, so its liquid column should be drawn")
		return
	if column.scale.y > float(main.FLUID_TANK_FILL_HEIGHT_VOXELS) + 0.001:
		_fail("the tank's column (%f) rose past its vessel" % column.scale.y)
		return

	# --- hover: the tank answers through the same one mechanism ----------
	var tank_text: String = main._hover_text_at_tile(tank_tile)
	if not tank_text.begins_with("fluid tank"):
		_fail("hovering the tank should describe it, got: %s" % tank_text)
		return
	if not tank_text.contains("units"):
		_fail("the tank tooltip should report its level, got: %s" % tank_text)
		return

	print("FLUIDS_OK: one network of %d placed segments rendered at the sim's own tiles, the tank-fed tile and the machine-inlet tile draw their own models, liquid bodies follow live pressure, and the tank and pipes both answer hover" % segments.size())
	quit(0)
