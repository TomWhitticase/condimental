extends Node2D
## Milestone 4 (ARCHITECTURE.md §9 item 4) presentation proof: renders the
## full, multi-stage production chain `SimBridge`/`sim_core::World` builds
## internally — however many stages `data/recipes.ron` currently defines —
## driven entirely by `chain_layout_json` (static per-stage geometry, read
## once) plus `chunk_snapshot_json`/`dirty_chunk_ids` (dynamic per-tick
## state). This script never hardcodes stage count or tile positions, and
## never reads or assumes anything about a belt's gap array or a machine's
## raw `TimedProcess` — only the JSON snapshot shapes (ARCHITECTURE.md
## §4/§5). Godot draws what the sim says exists; it doesn't maintain a
## second copy of the chain's layout.
##
## Visuals are flat placeholder textures (see placeholder_art/README.md),
## not the real voxel/isometric pipeline from UI_UX.md — this scene exists
## to prove the sim<->render wiring, not to be the final look.
##
## Milestone 5 (Economy MVP, ARCHITECTURE.md §9 item 5) added a minimal money
## + contract-status HUD line, read every frame from `economy_snapshot_json`
## — a plain Label, not UI_UX.md §5's clipboard/checklist diegetic styling
## yet (that's later polish).

const TILE_SIZE := 64
const TICK_HZ := 30.0

var sim: SimBridge
var _tick_accum := 0.0
var _item_sprites: Dictionary = {}
var _stage_order: Array[String] = [] # recipe ids, in chain order — for stable HUD line ordering
var _last_machines: Dictionary = {} # recipe id -> last machine snapshot dict
var _machine_label: Label
var _economy_label: Label
var _chain_tile_width := 5 # updated once chain_layout_json is read, in _build_tilemap


func _ready() -> void:
	sim = SimBridge.new()
	_build_tilemap()
	_build_camera()
	_build_ui()
	_apply_dirty_chunks() # render tick 0's (empty) state immediately


func _build_tilemap() -> void:
	var tileset := TileSet.new()
	tileset.tile_size = Vector2i(TILE_SIZE, TILE_SIZE)

	var belt_source := TileSetAtlasSource.new()
	belt_source.texture = load("res://placeholder_art/belt_tile.png")
	belt_source.texture_region_size = Vector2i(TILE_SIZE, TILE_SIZE)
	belt_source.create_tile(Vector2i(0, 0))
	var belt_source_id := tileset.add_source(belt_source)

	var machine_source := TileSetAtlasSource.new()
	machine_source.texture = load("res://placeholder_art/machine_tile.png")
	machine_source.texture_region_size = Vector2i(TILE_SIZE, TILE_SIZE)
	machine_source.create_tile(Vector2i(0, 0))
	var machine_source_id := tileset.add_source(machine_source)

	var tile_map := TileMap.new()
	tile_map.tile_set = tileset
	add_child(tile_map)

	# Ask the sim for its own layout rather than assuming stage count/size —
	# this is what lets the chain grow (a data-file-only recipe addition)
	# without any Godot-side change.
	var layout: Array = JSON.parse_string(sim.chain_layout_json())
	for stage in layout:
		_stage_order.append(stage["recipe"])
		var start: Array = stage["belt_start_tile"]
		var length: int = int(stage["belt_length_tiles"])
		for i in range(length):
			tile_map.set_cell(0, Vector2i(int(start[0]) + i, int(start[1])), belt_source_id, Vector2i(0, 0))
		var machine_tile: Array = stage["machine_tile"]
		tile_map.set_cell(0, Vector2i(int(machine_tile[0]), int(machine_tile[1])), machine_source_id, Vector2i(0, 0))

	if layout.size() > 0:
		var last_machine_tile: Array = layout[layout.size() - 1]["machine_tile"]
		_chain_tile_width = int(last_machine_tile[0]) + 1


func _build_camera() -> void:
	var camera := Camera2D.new()
	camera.position = Vector2(0.5 * _chain_tile_width * TILE_SIZE, 0.4 * TILE_SIZE)
	# Zoom out further for a wider chain so the whole line stays on screen.
	var zoom_fit: float = clamp(6.0 / max(_chain_tile_width, 1), 0.35, 1.4)
	camera.zoom = Vector2(zoom_fit, zoom_fit)
	camera.enabled = true
	add_child(camera)
	# Deferred: `make_current` needs the camera fully inside the tree,
	# which isn't guaranteed synchronously right after `add_child`.
	camera.call_deferred("make_current")


func _build_ui() -> void:
	var hud := CanvasLayer.new()
	add_child(hud)

	_machine_label = Label.new()
	_machine_label.position = Vector2(16, 16)
	hud.add_child(_machine_label)

	# Milestone 5 (Economy MVP, ARCHITECTURE.md §9 item 5): a minimal money +
	# contract-status readout. Deliberately a plain Label, not UI_UX.md §5's
	# clipboard/checklist diegetic styling — that's later visual polish, this
	# just needs to prove money/contract state is visible end to end.
	_economy_label = Label.new()
	_economy_label.position = Vector2(16, 16 + 20.0 * (_stage_order.size() + 1))
	hud.add_child(_economy_label)

	var help := Label.new()
	help.text = "SPACE: add a tomato to the start of the line    C: collect the finished product at the end"
	help.position = Vector2(16, 16 + 20.0 * (_stage_order.size() + 3))
	hud.add_child(help)


func _process(delta: float) -> void:
	_tick_accum += delta
	var tick_interval := 1.0 / TICK_HZ
	while _tick_accum >= tick_interval:
		_tick_accum -= tick_interval
		sim.tick()
		_apply_dirty_chunks()

	_update_machine_label()
	_update_economy_label()


func _unhandled_input(event: InputEvent) -> void:
	if not (event is InputEventKey and event.pressed and not event.echo):
		return

	var command := ""
	if event.keycode == KEY_SPACE:
		command = JSON.stringify({"type": "spawn_item", "item": "tomato"})
	elif event.keycode == KEY_C:
		command = JSON.stringify({"type": "collect_output"})
	else:
		return

	sim.issue_command_json(command)
	# Render immediately rather than waiting for the next scheduled tick —
	# issue_command's dirty marking isn't cleared until tick() runs next,
	# so this is safe to read right away.
	_apply_dirty_chunks()


func _apply_dirty_chunks() -> void:
	var dirty: PackedInt64Array = sim.dirty_chunk_ids()
	for chunk_id in dirty:
		var parsed = JSON.parse_string(sim.chunk_snapshot_json(chunk_id))
		if parsed != null:
			_render_snapshot(parsed)


func _render_snapshot(snapshot: Dictionary) -> void:
	var seen := {}

	var belts: Array = snapshot.get("belts", [])
	for belt in belts:
		var start: Array = belt["start_tile"]
		var items: Array = belt["items"]
		for i in range(items.size()):
			var pair: Array = items[i]
			# Keyed by belt start tile so multiple stages' belts never
			# collide with each other's item sprites.
			var key := "belt_%d_%d_%d" % [int(start[0]), int(start[1]), i]
			seen[key] = true
			var sprite := _sprite_for(key, pair[0])
			sprite.visible = true
			sprite.position = Vector2(
				(float(start[0]) + float(pair[1]) + 0.5) * TILE_SIZE,
				(float(start[1]) + 0.5) * TILE_SIZE
			)

	for key in _item_sprites.keys():
		if not seen.has(key):
			_item_sprites[key].visible = false

	var machines: Array = snapshot.get("machines", [])
	for machine in machines:
		_last_machines[machine["recipe"]] = machine


func _sprite_for(key: String, item_name: String) -> Sprite2D:
	if _item_sprites.has(key):
		return _item_sprites[key]
	var sprite := Sprite2D.new()
	sprite.texture = _texture_for_item(item_name)
	add_child(sprite)
	_item_sprites[key] = sprite
	return sprite


func _texture_for_item(item_name: String) -> Texture2D:
	if item_name == "tomato":
		return load("res://placeholder_art/item_tomato.png")
	# Every processed intermediate/final item shares the placeholder "sauce"
	# sprite for now — distinct per-item art is UI_UX.md's voxel pipeline's
	# job, not this milestone's.
	return load("res://placeholder_art/item_sauce.png")


func _update_machine_label() -> void:
	var lines: Array[String] = []
	for recipe_id in _stage_order:
		if not _last_machines.has(recipe_id):
			continue
		var machine: Dictionary = _last_machines[recipe_id]
		var process = machine.get("process")
		var progress_text := "idle"
		if process != null:
			var elapsed: int = sim.current_tick() - int(process[0])
			var required: int = int(process[1])
			var pct: float = clamp(float(elapsed) / float(required), 0.0, 1.0) * 100.0
			progress_text = "%.0f%%" % pct
		lines.append("%s — %s | in: %s | out: %s" % [
			recipe_id,
			progress_text,
			_format_counts(machine["input"]),
			_format_counts(machine["output"]),
		])

	_machine_label.text = "\n".join(lines)


func _update_economy_label() -> void:
	var econ: Dictionary = JSON.parse_string(sim.economy_snapshot_json())
	var money: float = float(econ["money"])
	var mix: Array = econ["contract_mix"]
	var progress: Array = econ["contract_progress"]
	var mix_parts: Array[String] = []
	for i in range(mix.size()):
		var required_pair: Array = mix[i]
		var have: int = int(progress[i][1])
		mix_parts.append("%s %d/%d" % [required_pair[0], have, int(required_pair[1])])

	var ticks_left: int = max(0, int(econ["next_delivery_tick"]) - sim.current_tick())
	_economy_label.text = "Money: %.0f    Contract: %s (next check in %d ticks)" % [
		money, ", ".join(mix_parts), ticks_left,
	]


func _format_counts(pairs: Array) -> String:
	if pairs.is_empty():
		return "-"
	var parts: Array[String] = []
	for pair in pairs:
		parts.append("%s x%d" % [pair[0], int(pair[1])])
	return ", ".join(parts)
