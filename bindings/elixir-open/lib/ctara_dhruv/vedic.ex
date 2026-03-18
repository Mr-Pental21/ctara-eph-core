defmodule CtaraDhruv.Vedic do
  @moduledoc """
  Low-level Vedic operations.

  `lagna/2`, `mc/2`, and `bhavas/2` return the direct bhava-surface output for
  the supplied request. When the request omits `:sankranti_config`, those
  values are tropical. Use the `/3` convenience arities to attach an explicit
  sidereal ayanamsha config at the wrapper layer.
  """

  alias CtaraDhruv.Native

  def ayanamsha(engine, request),
    do: Native.call_engine(&Native.vedic_run/2, engine, Map.put(request, :op, :ayanamsha))

  def lunar_node(engine, request),
    do: Native.call_engine(&Native.vedic_run/2, engine, Map.put(request, :op, :lunar_node))

  def rise_set(engine, request),
    do: Native.call_engine(&Native.vedic_run/2, engine, Map.put(request, :op, :rise_set))

  def all_events(engine, request),
    do: Native.call_engine(&Native.vedic_run/2, engine, Map.put(request, :op, :all_events))

  def lagna(engine, request),
    do: Native.call_engine(&Native.vedic_run/2, engine, Map.put(request, :op, :lagna))

  def lagna(engine, request, sankranti_config),
    do: lagna(engine, put_sankranti_config(request, sankranti_config))

  def mc(engine, request),
    do: Native.call_engine(&Native.vedic_run/2, engine, Map.put(request, :op, :mc))

  def mc(engine, request, sankranti_config),
    do: mc(engine, put_sankranti_config(request, sankranti_config))

  def ramc(engine, request),
    do: Native.call_engine(&Native.vedic_run/2, engine, Map.put(request, :op, :ramc))

  def bhavas(engine, request),
    do: Native.call_engine(&Native.vedic_run/2, engine, Map.put(request, :op, :bhavas))

  def bhavas(engine, request, sankranti_config),
    do: bhavas(engine, put_sankranti_config(request, sankranti_config))

  defp put_sankranti_config(request, sankranti_config),
    do: Map.put(request, :sankranti_config, sankranti_config)
end
