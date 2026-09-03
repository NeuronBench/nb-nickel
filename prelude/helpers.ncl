# Hand-written helpers exposed on the `nb` prelude record. The generated
# schema contracts (schema.ncl) are merged in alongside these.
{
  # `Nullable C` accepts `null` or a value satisfying `C`. Mirrors `Option<T>`.
  Nullable = fun C => std.contract.custom (fun label value =>
    if value == null then 'Ok value else 'Ok (std.contract.apply C label value)),

  # `Tagged { A = {...}, B = {...} }` checks a record whose `type` field names
  # one of the variants, then applies that variant's record contract. This is
  # the shape serde produces for `#[serde(tag = "type")]` enums. The `type`
  # field may be an enum tag (`'Gaussian`) or a string (`"Gaussian"`).
  Tagged = fun variants => std.contract.custom (fun label value =>
    let expected = std.string.join ", " (std.record.fields variants) in
    if !std.is_record value then
      'Error { message = "expected a record with a `type` field (one of %{expected})" }
    else if !std.record.has_field "type" value then
      'Error { message = "missing `type` field; expected one of %{expected}" }
    else
      let tag =
        if std.is_enum value.type then std.to_string value.type
        else if std.is_string value.type then value.type
        else "" in
      if std.record.has_field tag variants then
        'Ok (std.contract.apply variants."%{tag}" label value)
      else
        'Error { message = "unknown type `%{tag}`; expected one of %{expected}" }),

  # `Slider { min, max }` bounds a numeric parameter. nb-sim renders fields
  # of `params` that carry this contract as sliders in the UI.
  Slider = fun spec => std.contract.custom (fun label value =>
    if !std.is_number value then
      'Error { message = "slider parameters must be numbers" }
    else if value < spec.min || value > spec.max then
      'Error { message = "%{std.to_string value} is outside the slider range [%{std.to_string spec.min}, %{std.to_string spec.max}]" }
    else 'Ok value),
}
