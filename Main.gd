extends Node2D

@export var falling_object_scene: PackedScene

var bananas: int = 0

func _ready() -> void:
	var initialize_response: Dictionary = Steam.steamInitEx()
	$SteamStatus.text=str(initialize_response)
	print("Did Steam initialize?: %s " % initialize_response)
	
	$Button.connect("pressed", Callable(self, "_on_button_pressed"))
	$SubViewportContainer/Label.text = "bananas: " + str(bananas)

func _on_button_pressed():
	bananas += 1
	$SubViewportContainer/Label.text = "bananas: " + str(bananas)
	
	var instance = falling_object_scene.instantiate()
	
	instance.position = Vector2(randf_range(0, get_viewport_rect().size.x), 0)
	
	add_child(instance)

# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(_delta: float) -> void:
	pass


func _on_button_2_pressed() -> void:
	print ("button pressed")
	var device := 0 # The joystick device index, change as needed
	var weak_magnitude := float($Weak.text) # Weak vibration strength (0 to 1)
	var strong_magnitude := float($Strong.text)  # Strong vibration strength (0 to 1)
	var duration := float($Duration.text)  # Duration in seconds

	# Trigger joystick vibration
	Input.start_joy_vibration(device, weak_magnitude, strong_magnitude, duration)
