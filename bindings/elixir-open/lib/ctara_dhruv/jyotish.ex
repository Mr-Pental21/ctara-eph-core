defmodule CtaraDhruv.Jyotish do
  @moduledoc """
  Jyotish chart computations.

  `full_kundali/2` uses the request's `:sankranti_config` when provided and the
  wrapper's resolved ayanamsha defaults otherwise. `full_kundali/3` is a
  convenience arity for explicitly supplying the chart ayanamsha config from
  Elixir.
  """

  alias CtaraDhruv.Native

  def graha_longitudes(engine, request),
    do:
      Native.call_engine(&Native.jyotish_run/2, engine, Map.put(request, :op, :graha_longitudes))

  def graha_positions(engine, request),
    do: Native.call_engine(&Native.jyotish_run/2, engine, Map.put(request, :op, :graha_positions))

  def special_lagnas(engine, request),
    do: Native.call_engine(&Native.jyotish_run/2, engine, Map.put(request, :op, :special_lagnas))

  def arudha(engine, request),
    do: Native.call_engine(&Native.jyotish_run/2, engine, Map.put(request, :op, :arudha))

  def upagrahas(engine, request),
    do: Native.call_engine(&Native.jyotish_run/2, engine, Map.put(request, :op, :upagrahas))

  def bindus(engine, request),
    do: Native.call_engine(&Native.jyotish_run/2, engine, Map.put(request, :op, :bindus))

  def ashtakavarga(engine, request),
    do: Native.call_engine(&Native.jyotish_run/2, engine, Map.put(request, :op, :ashtakavarga))

  def drishti(engine, request),
    do: Native.call_engine(&Native.jyotish_run/2, engine, Map.put(request, :op, :drishti))

  def charakaraka(engine, request),
    do: Native.call_engine(&Native.jyotish_run/2, engine, Map.put(request, :op, :charakaraka))

  def shadbala(engine, request),
    do: Native.call_engine(&Native.jyotish_run/2, engine, Map.put(request, :op, :shadbala))

  def bhavabala(engine, request),
    do: Native.call_engine(&Native.jyotish_run/2, engine, Map.put(request, :op, :bhavabala))

  def vimsopaka(engine, request),
    do: Native.call_engine(&Native.jyotish_run/2, engine, Map.put(request, :op, :vimsopaka))

  def balas(engine, request),
    do: Native.call_engine(&Native.jyotish_run/2, engine, Map.put(request, :op, :balas))

  def avastha(engine, request),
    do: Native.call_engine(&Native.jyotish_run/2, engine, Map.put(request, :op, :avastha))

  def full_kundali(engine, request),
    do: Native.call_engine(&Native.jyotish_run/2, engine, Map.put(request, :op, :full_kundali))

  def full_kundali(engine, request, sankranti_config),
    do: full_kundali(engine, put_sankranti_config(request, sankranti_config))

  def amsha(engine, request),
    do: Native.call_engine(&Native.jyotish_run/2, engine, Map.put(request, :op, :amsha))

  defp put_sankranti_config(request, sankranti_config),
    do: Map.put(request, :sankranti_config, sankranti_config)
end
