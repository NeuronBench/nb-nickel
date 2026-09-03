# GENERATED FILE. Do not edit by hand.
# Source of truth: src/serialize.rs in nb-sim.
# Regenerate with: cargo run --bin gen_nickel_schema
fun helpers => {
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

  Neuron = {
    membranes | Array (Membrane),
    segments | Array (Segment),
  },

  Receptor = {
    membrane_channel | MembraneChannel,
    neurotransmitter_sensitivity | Sensitivity,
  },

  Scene = {
    neurons | Array (SceneNeuron),
    synapses | Array (Synapse),
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
