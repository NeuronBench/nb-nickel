# GENERATED FILE. Do not edit by hand.
# Source of truth: src/serialize.rs in nb-sim.
# Regenerate with: cargo run --bin gen_nickel_schema
fun helpers => {
  # A grid of current injections, one cell per segment, painted with the mouse in the Canvas window: left-drag raises a cell, right-drag lowers it. Cells range over -1..1; a cell at 1 injects `max_uamps_per_square_cm` into its segment.
  Canvas = {
    height | std.number.Integer,
    initial | Array (Number) | doc "Starting cell values in -1..1, row-major; empty for all zeros." | optional,
    max_uamps_per_square_cm | Number,
    name | String | doc "Shown as the window title; the painting is kept across reloads while the name and size stay the same.",
    taps | Array (SegmentRef) | doc "The segment under each cell, row-major (`width * height` of them).",
    width | std.number.Integer,
  },

  Channel = {
    activation | helpers.Nullable (GatingParameters) | optional,
    inactivation | helpers.Nullable (GatingParameters) | optional,
    ion_selectivity | IonSelectivity,
  },

  CurrentShape = helpers.Tagged {
    SquareWave = {
      type | Dyn,
      off_current_uamps_per_square_cm | Number,
      on_current_uamps_per_square_cm | Number,
    },
    LinearRamp = {
      type | Dyn,
      end_current_uamps_per_square_cm | Number,
      off_current_uamps_per_square_cm | Number,
      start_current_uamps_per_square_cm | Number,
    },
    FrequencyRamp = {
      type | Dyn,
      end_frequency_hz | Number,
      offset_current_uamps_per_square_cm | Number,
      on_amplitude_uamps_per_square_cm | Number,
      start_frequency_hz | Number,
    },
  },

  Envelope = {
    offset_sec | Number,
    onset_sec | Number,
    period_sec | Number,
  },

  GatingParameters = {
    gates | std.number.Integer,
    magnitude | Magnitude,
    time_constant | TimeConstant,
  },

  Gaussian = {
    max_molar | Number,
    min_molar | Number,
    slope | Number,
    v_at_half_max_mv | Number,
  },

  # Drive the simulation from part of an intermediate value of a model.
  HiddenTap = {
    elements | Array (std.number.Integer) | doc "Flat (row-major) indices into the value: element `elements[i]` drives target i of `output`.",
    output | ModelOutput,
    value | String | doc "The value's name, as the Models window shows it (`conv2d1`, `relu2`).",
  },

  IonSelectivity = {
    ca | Number,
    cl | Number,
    k | Number,
    na | Number,
  },

  Location = {
    x_mm | Number,
    y_mm | Number,
    z_mm | Number,
  },

  Magnitude = {
    slope | Number,
    v_at_half_max_mv | Number,
  },

  Membrane = {
    capacitance_farads_per_square_cm | Number,
    membrane_channels | Array (MembraneChannel),
  },

  MembraneChannel = {
    channel | Channel,
    siemens_per_square_cm | Number,
  },

  # Where a model input's values come from. Each fills the input tensor in row-major order.
  ModelInput = helpers.Tagged {
    Voltages = {
      type | Dyn,
      offset_mv | Number,
      scale | Number,
      taps | Array (SegmentRef),
    },
    Time = {
      type | Dyn,
    },
    Constant = {
      type | Dyn,
      values | Array (Number),
    },
    Feedback = {
      type | Dyn,
      output | String,
    },
  },

  # What a model output drives.
  ModelOutput = helpers.Tagged {
    InjectCurrent = {
      type | Dyn,
      scale_uamps_per_square_cm | Number,
      taps | Array (SegmentRef),
    },
    ScaleConductance = {
      type | Dyn,
      channel | helpers.Nullable (std.number.Integer) | optional,
      offset | Number,
      scale | Number,
      taps | Array (SegmentRef),
    },
    SynapticGain = {
      type | Dyn,
      offset | Number,
      scale | Number,
      synapses | Array (std.number.Integer),
    },
    Ignore = {
      type | Dyn,
    },
  },

  Neuron = {
    membranes | Array (Membrane),
    segments | Array (Segment),
  },

  Receptor = {
    membrane_channel | MembraneChannel,
    neurotransmitter_sensitivity | Sensitivity,
  },

  Scene = {
    canvases | Array (Canvas) | doc "Grids of injected current painted in the Canvas window." | optional,
    models | Array (SceneModel) | doc "ONNX models coupled to the simulation." | optional,
    neurons | Array (SceneNeuron),
    synapses | Array (Synapse),
  },

  # An ONNX model that reads simulation state and drives the simulation. Graph inputs and outputs bind, in order, to `inputs` and `outputs`.
  SceneModel = {
    hidden | Array (HiddenTap) | doc "Extra sinks fed from intermediate values (hidden layers)." | optional,
    inputs | Array (ModelInput) | doc "One entry per graph input, in graph order.",
    interval_sec | Number | doc "Simulated seconds between evaluations. Models run at most once per frame.",
    outputs | Array (ModelOutput) | doc "One entry per graph output, in graph order.",
    url | String | doc "The `.onnx` file: a URL, or a path relative to the root scene file.",
  },

  SceneNeuron = {
    location | Location,
    neuron | Neuron,
    stimulator_segments | Array (StimulatorSegment),
  },

  Segment = {
    id | std.number.Integer,
    parent | std.number.Integer,
    r | Number,
    type | std.number.Integer,
    x | Number,
    y | Number,
    z | Number,
  },

  # A segment in the scene: an index into `neurons`, and the segment's SWC id (as in `stimulator_segments`).
  SegmentRef = {
    neuron | std.number.Integer,
    segment | std.number.Integer,
  },

  Sensitivity = {
    concentration_at_half_max_molar | Number,
    slope | Number,
    transmitter | String,
  },

  Solution = {
    ca | Number,
    cl | Number,
    k | Number,
    na | Number,
  },

  Stimulator = {
    current_shape | CurrentShape,
    envelope | Envelope,
  },

  StimulatorSegment = {
    segment | std.number.Integer,
    stimulator | Stimulator,
  },

  Synapse = {
    post_neuron | std.number.Integer,
    post_segment | std.number.Integer,
    pre_neuron | std.number.Integer,
    pre_segment | std.number.Integer,
    synapse_membranes | SynapseMembranes,
  },

  SynapseMembranes = {
    cleft_solution | Solution,
    postsynaptic_receptors | Array (Receptor),
    presynaptic_pumps | Array (TransmitterPump),
    surface_area_square_mm | Number,
    transmitter_concentrations | TransmitterConcentrations,
  },

  TimeConstant = helpers.Tagged {
    Instantaneous = {
      type | Dyn,
    },
    Gaussian = {
      type | Dyn,
      c_amp | Number,
      c_base | Number,
      sigma | Number,
      v_at_max_tau_mv | Number,
    },
    LinearExp = {
      type | Dyn,
      coef | Number,
      inner_coef | Number,
      v_offset_mv | Number,
    },
  },

  TransmitterConcentrations = {
    gaba_molar | Number,
    glutamate_molar | Number,
  },

  TransmitterPump = {
    transmitter | String,
    transmitter_pump_params | TransmitterPumpParams,
  },

  TransmitterPumpParams = {
    target_concentration | Gaussian,
    time_constant | TimeConstant,
  },

}
