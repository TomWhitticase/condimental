extends Node2D
## Milestone 3 (ARCHITECTURE.md §9 item 3) presentation proof: renders the
## fixed belt+machine layout `SimBridge`/`sim_core::World` builds internally,
## driven entirely by `chunk_snapshot_json`/`dirty_chunk_ids` — this script
## never reads or assumes anything about the belt's gap array or the
## machine's raw `TimedProcess`, only the JSON snapshot shape
## (ARCHITECTURE.md §4/§5).
##
## Visuals are flat placeholder textures (see placeholder_art/README.md),
## not the real voxel/isometric pipeline from UI_UX.md — this scene exists
## to prove the sim<->render wiring, not to be the final look.

const TILE_SIZE := 64
const TICK_HZ := 30.0

var sim: SimBridge
var _tick_accum := 0.0
var _item_sprites: Dictionary = {}
var _machine_label: Label
var _last_machine = null


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

	# Mirrors sim_core::World::new's fixed milestone-3 layout exactly
	# (4-tile belt at y=0, machine at the next tile over). Godot draws what
	# the sim says exists; it doesn't maintain a second copy of the layout.
	for x in range(4):
		tile_map.set_cell(0, Vector2i(x, 0), belt_source_id, Vector2i(0, 0))
	tile_map.set_cell(0, Vector2i(4, 0), machine_source_id, Vector2i(0, 0))


func _build_camera() -> void:
	var camera := Camera2D.new()
	camera.position = Vector2(2.0 * TILE_SIZE, 0.4 * TILE_SIZE)
	camera.zoom = Vector2(1.4, 1.4)
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

	var help := Label.new()
	help.text = "SPACE: add a tomato to the belt    C: collect finished sauce"
	help.position = Vector2(16, 100)
	hud.add_child(help)


func _process(delta: float) -> void:
	_tick_accum += delta
	var tick_interval := 1.0 / TICK_HZ
	while _tick_accum >= tick_interval:
		_tick_accum -= tick_interval
		sim.tick()
		_apply_dirty_chunks()

	_update_machine_label()


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

	var belt = snapshot.get("belt")
	if belt != null:
		var start: Array = belt["start_tile"]
		var items: Array = belt["items"]
		for i in range(items.size()):
			var pair: Array = items[i]
			var key := "belt_%d" % i
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

	var machine = snapshot.get("machine")
	if machine != null:
		_last_machine = machine


func _sprite_for(key: String, item_name: String) -> Sprite2D:
	if _item_sprites.has(key):
		return _item_sprites[key]
	var sprite := Sprite2D.new()
	sprite.texture = _texture_for_item(item_name)
	add_child(sprite)
	_item_sprites[key] = sprite
	return sprite


func _texture_for_item(item_name: String) -> Texture2D:
	if item_name == "tomato_sauce":
		return load("res://placeholder_art/item_sauce.png")
	return load("res://placeholder_art/item_tomato.png")


func _update_machine_label() -> void:
	if _last_machine == null:
		return

	var process = _last_machine.get("process")
	var progress_text := "idle"
	if process != null:
		var elapsed: int = sim.current_tick() - int(process[0])
		var required: int = int(process[1])
		var pct: float = clamp(float(elapsed) / float(required), 0.0, 1.0) * 100.0
		progress_text = "%.0f%%" % pct

	_machine_label.text = "Recipe: %s\nProgress: %s\nInput: %s\nOutput: %s" % [
		_last_machine["recipe"],
		progress_text,
		_format_counts(_last_machine["input"]),
		_format_counts(_last_machine["output"]),
	]


func _format_counts(pairs: Array) -> String:
	if pairs.is_empty():
		return "-"
	var parts: Array[String] = []
	for pair in pairs:
		parts.append("%s x%d" % [pair[0], int(pair[1])])
	return ", ".join(parts)
