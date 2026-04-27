defmodule CtaraDhruv.Native do
  @moduledoc false
  @enum_keys [
    :accuracy,
    :ayanamsha,
    :backend,
    :body,
    :event,
    :graha,
    :grahan_type,
    :hora,
    :kind,
    :masa,
    :mode,
    :nakshatra,
    :nature,
    :node,
    :output,
    :paksha,
    :policy,
    :phase,
    :rashi,
    :role,
    :gender,
    :dignity,
    :relationship,
    :samvatsara,
    :source,
    :station_type,
    :status,
    :system,
    :type,
    :vaar,
    :yoga
  ]

  use Rustler,
    otp_app: :ctara_dhruv,
    crate: "dhruv_elixir_nif",
    path: "native/dhruv_elixir_nif",
    mode: if(Mix.env() == :prod, do: :release, else: :debug)

  alias CtaraDhruv.Engine
  alias CtaraDhruv.Error

  def engine_new(_config), do: :erlang.nif_error(:nif_not_loaded)
  def engine_close(_resource), do: :erlang.nif_error(:nif_not_loaded)
  def engine_load_config(_resource, _request), do: :erlang.nif_error(:nif_not_loaded)
  def engine_clear_config(_resource), do: :erlang.nif_error(:nif_not_loaded)
  def engine_replace_spks(_resource, _request), do: :erlang.nif_error(:nif_not_loaded)
  def engine_list_spks(_resource), do: :erlang.nif_error(:nif_not_loaded)
  def engine_load_eop(_resource, _request), do: :erlang.nif_error(:nif_not_loaded)
  def engine_clear_eop(_resource), do: :erlang.nif_error(:nif_not_loaded)
  def engine_load_tara_catalog(_resource, _request), do: :erlang.nif_error(:nif_not_loaded)
  def engine_reset_tara_catalog(_resource), do: :erlang.nif_error(:nif_not_loaded)
  def ephemeris_run(_resource, _request), do: :erlang.nif_error(:nif_not_loaded)
  def time_run(_resource, _request), do: :erlang.nif_error(:nif_not_loaded)
  def util_run(_request), do: :erlang.nif_error(:nif_not_loaded)
  def vedic_run(_resource, _request), do: :erlang.nif_error(:nif_not_loaded)
  def panchang_run(_resource, _request), do: :erlang.nif_error(:nif_not_loaded)
  def search_run(_resource, _request), do: :erlang.nif_error(:nif_not_loaded)
  def jyotish_run(_resource, _request), do: :erlang.nif_error(:nif_not_loaded)
  def dasha_run(_resource, _request), do: :erlang.nif_error(:nif_not_loaded)
  def tara_run(_resource, _request), do: :erlang.nif_error(:nif_not_loaded)

  def create_engine(config), do: handle(engine_new(normalize_term(config)))

  def call_engine_noarg(fun, %Engine{} = engine), do: handle(fun.(engine.resource))

  def call_engine(fun, %Engine{} = engine, request),
    do: handle(fun.(engine.resource, normalize_term(request)))

  def call_util(fun, request), do: handle(fun.(normalize_term(request)))

  defp handle({:ok, result}), do: {:ok, postprocess(result)}
  defp handle({:error, %{} = error}), do: {:error, Error.from_term(postprocess(error))}

  defp normalize_term(term) when is_atom(term) and term not in [true, false, nil],
    do: Atom.to_string(term)

  defp normalize_term(%_{} = struct), do: struct |> Map.from_struct() |> normalize_term()

  defp normalize_term(%{} = map),
    do: Map.new(map, fn {k, v} -> {normalize_key(k), normalize_term(v)} end)

  defp normalize_term(list) when is_list(list), do: Enum.map(list, &normalize_term/1)
  defp normalize_term(term), do: term

  defp normalize_key(key) when is_atom(key), do: Atom.to_string(key)
  defp normalize_key(key), do: key

  defp postprocess(%{} = map) do
    Map.new(map, fn
      {:__struct__, _} -> {:__struct__, nil}
      {key, value} -> {atomize_key(key), postprocess_value(atomize_key(key), value)}
    end)
    |> Map.delete(:__struct__)
  end

  defp postprocess(list) when is_list(list), do: Enum.map(list, &postprocess/1)
  defp postprocess(other), do: other

  defp postprocess_value(_key, %{} = value), do: postprocess(value)
  defp postprocess_value(_key, list) when is_list(list), do: Enum.map(list, &postprocess/1)

  defp postprocess_value(key, value) when is_binary(value) and key in @enum_keys do
    String.to_atom(value)
  end

  defp postprocess_value(_key, value), do: value

  defp atomize_key(key) when is_atom(key), do: key
  defp atomize_key(key) when is_binary(key), do: String.to_atom(key)
end
