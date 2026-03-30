defmodule CtaraDhruv.Engine do
  @moduledoc """
  Engine lifecycle and mutable wrapper state.
  """

  alias CtaraDhruv.Native

  @enforce_keys [:resource]
  defstruct [:resource]

  @type t :: %__MODULE__{resource: reference()}

  def new(config) when is_map(config) do
    with {:ok, resource} <- Native.create_engine(config) do
      {:ok, %__MODULE__{resource: resource}}
    end
  end

  def close(%__MODULE__{} = engine), do: Native.call_engine_noarg(&Native.engine_close/1, engine)

  def load_config(%__MODULE__{} = engine, path) when is_binary(path),
    do: Native.call_engine(&Native.engine_load_config/2, engine, %{path: path})

  def load_config(%__MODULE__{} = engine, request) when is_map(request),
    do: Native.call_engine(&Native.engine_load_config/2, engine, request)

  def clear_config(%__MODULE__{} = engine),
    do: Native.call_engine_noarg(&Native.engine_clear_config/1, engine)

  def load_eop(%__MODULE__{} = engine, path),
    do: Native.call_engine(&Native.engine_load_eop/2, engine, %{path: path})

  def clear_eop(%__MODULE__{} = engine),
    do: Native.call_engine_noarg(&Native.engine_clear_eop/1, engine)

  def load_tara_catalog(%__MODULE__{} = engine, path),
    do: Native.call_engine(&Native.engine_load_tara_catalog/2, engine, %{path: path})

  def reset_tara_catalog(%__MODULE__{} = engine),
    do: Native.call_engine_noarg(&Native.engine_reset_tara_catalog/1, engine)
end
