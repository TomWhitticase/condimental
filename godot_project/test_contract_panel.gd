# Headless structural proof of the contract "clipboard" panel (docs/UI_UX.md
# §5 — "a contract IS a clipboard"), which reskinned the plain money/contract
# Label into its own diegetic panel with checkbox-style tick marks.
#
# Run with:
#   godot --headless --path godot_project --script test_contract_panel.gd
#
# Like test_presentation.gd, this deliberately does NOT claim to verify that
# the panel *looks* like a clipboard — that needs eyes on
# scripts/screenshot-scene.sh's output. What it verifies is the structural
# half:
#
#   * the economy/contract readout lives in its own distinct node
#     (`ContractPanel`), separate from whatever the general HUD panel above
#     it contains — so this stays intact regardless of what a sibling ticket
#     does to that other panel,
#   * at least one checkbox-representing node exists per required unit of the
#     hardcoded contract's mix, read from the real `economy_snapshot_json`
#     rather than a hardcoded assumption about what the contract asks for,
#   * a checkbox's visual state (its stylebox override + tick-mark
#     visibility) actually changes when `contract_progress` changes — driven
#     directly through `_set_checkbox_checked`, the same "call the method
#     directly, no live mouse/window needed" approach test_camera_nav.gd uses.
extends SceneTree


func _fail(msg: String) -> void:
	printerr("CONTRACT_PANEL_FAIL: %s" % msg)
	quit(1)


func _initialize() -> void:
	var main = load("res://main.tscn").instantiate()
	root.add_child(main)
	main._ready()

	# --- The panel is its own distinct node, not co-mingled Labels --------
	var hud: Node = main.get_node_or_null("HUD")
	if hud == null:
		_fail("no HUD CanvasLayer found")
		return

	var contract_panel: PanelContainer = main._contract_panel
	if contract_panel == null:
		_fail("main._contract_panel was not built")
		return
	if contract_panel.name != "ContractPanel":
		_fail("expected the contract panel's node name to be 'ContractPanel', was '%s'" % contract_panel.name)
		return
	if contract_panel.get_parent() != hud:
		_fail("ContractPanel must be a direct child of the HUD CanvasLayer, not nested inside another panel")
		return

	# It must be a genuinely separate node from the general HUD panel (the
	# one holding the per-stage machine lines) — not the same PanelContainer
	# ticket A's hover-tooltip work might reshape.
	var siblings: Array = hud.get_children()
	var other_panels := 0
	for child in siblings:
		if child is PanelContainer and child != contract_panel:
			other_panels += 1
	if other_panels < 1:
		_fail("expected at least one other HUD panel besides ContractPanel — did the general HUD panel disappear?")
		return

	# --- Checkbox nodes exist, one per required unit of the real contract --
	var econ: Dictionary = JSON.parse_string(main.sim.economy_snapshot_json())
	var mix: Array = econ.get("contract_mix", [])
	if mix.is_empty():
		_fail("economy_snapshot_json reported an empty contract_mix — nothing to check against")
		return

	var item_id: String = String(mix[0][0])
	var required: int = int(mix[0][1])
	var boxes: Array = main._contract_checklist_boxes.get(item_id, [])
	if boxes.is_empty():
		_fail("no checklist boxes built for contract item '%s'" % item_id)
		return

	var expected_box_count: int = required if required <= main.CONTRACT_CHECKLIST_MAX_BOXES else 1
	if boxes.size() != expected_box_count:
		_fail("expected %d checkbox node(s) for '%s' (required=%d), found %d" % [
			expected_box_count, item_id, required, boxes.size()
		])
		return

	for box in boxes:
		if not (box is Panel):
			_fail("checklist entries must be Panel nodes (the checkbox squares)")
			return

	# --- A checkbox's visual state actually changes with contract_progress -
	var box: Panel = boxes[0]
	var mark: Label = box.get_node_or_null("Mark")
	if mark == null:
		_fail("checkbox '%s' has no 'Mark' tick-mark child" % box.name)
		return

	var style_before = box.get_theme_stylebox("panel")
	var visible_before: bool = mark.visible

	main._set_checkbox_checked(box, true)
	if box.get_theme_stylebox("panel") == style_before:
		_fail("checking a box did not change its stylebox override")
		return
	if not mark.visible:
		_fail("checking a box did not reveal its tick mark")
		return
	if box.get_theme_stylebox("panel") != main._checkbox_style_checked:
		_fail("a checked box must use the checked stylebox resource")
		return

	main._set_checkbox_checked(box, false)
	if box.get_theme_stylebox("panel") != main._checkbox_style_unchecked:
		_fail("unchecking a box did not restore the unchecked stylebox")
		return
	if mark.visible:
		_fail("unchecking a box did not hide its tick mark")
		return

	# --- The full update path (economy_snapshot_json -> box state) works --
	main._update_contract_panel()
	var boxes_after_zero: Array = main._contract_checklist_boxes.get(item_id, [])
	for b in boxes_after_zero:
		if bool(b.get_meta("checked", false)):
			_fail("with zero contract_progress, no checkbox should read as checked yet")
			return

	print("CONTRACT_PANEL_OK: ContractPanel is a distinct node, %d checkbox(es) for '%s', checkbox visual state toggles correctly" % [
		boxes.size(), item_id
	])
	quit(0)
