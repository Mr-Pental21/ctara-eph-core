defmodule CtaraDhruv.Math do
  @moduledoc false

  alias CtaraDhruv.Native

  def rashi_from_longitude(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :rashi_from_longitude))

  def nakshatra_from_longitude(request),
    do:
      Native.call_util(
        &Native.util_run/1,
        Map.put(request, :op, :nakshatra_from_longitude)
      )

  def nakshatra28_from_longitude(request),
    do:
      Native.call_util(
        &Native.util_run/1,
        Map.put(request, :op, :nakshatra28_from_longitude)
      )

  def rashi_from_tropical(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :rashi_from_tropical))

  def nakshatra_from_tropical(request),
    do:
      Native.call_util(
        &Native.util_run/1,
        Map.put(request, :op, :nakshatra_from_tropical)
      )

  def nakshatra28_from_tropical(request),
    do:
      Native.call_util(
        &Native.util_run/1,
        Map.put(request, :op, :nakshatra28_from_tropical)
      )

  def graha_name(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :graha_name))

  def yogini_name(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :yogini_name))

  def rashi_name(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :rashi_name))

  def nakshatra_name(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :nakshatra_name))

  def nakshatra28_name(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :nakshatra28_name))

  def sphuta_name(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :sphuta_name))

  def upagraha_name(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :upagraha_name))

  def amsha_variations(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :amsha_variations))

  def amsha_variations_many(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :amsha_variations_many))

  def hora_lord(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :hora_lord))

  def masa_lord(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :masa_lord))

  def samvatsara_lord(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :samvatsara_lord))

  def exaltation_degree(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :exaltation_degree))

  def debilitation_degree(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :debilitation_degree))

  def moolatrikone_range(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :moolatrikone_range))

  def combustion_threshold(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :combustion_threshold))

  def combust?(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :is_combust))

  def all_combustion_status(request),
    do:
      Native.call_util(&Native.util_run/1, Map.put(request, :op, :all_combustion_status))

  def naisargika_maitri(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :naisargika_maitri))

  def tatkalika_maitri(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :tatkalika_maitri))

  def panchadha_maitri(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :panchadha_maitri))

  def dignity_in_rashi(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :dignity_in_rashi))

  def dignity_in_rashi_with_positions(request),
    do:
      Native.call_util(
        &Native.util_run/1,
        Map.put(request, :op, :dignity_in_rashi_with_positions)
      )

  def node_dignity_in_rashi(request),
    do:
      Native.call_util(&Native.util_run/1, Map.put(request, :op, :node_dignity_in_rashi))

  def natural_benefic_malefic(request),
    do:
      Native.call_util(
        &Native.util_run/1,
        Map.put(request, :op, :natural_benefic_malefic)
      )

  def moon_benefic_nature(request),
    do:
      Native.call_util(&Native.util_run/1, Map.put(request, :op, :moon_benefic_nature))

  def graha_gender(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :graha_gender))

  def graha_drishti(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :graha_drishti))

  def graha_drishti_matrix(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :graha_drishti_matrix))

  def sun_based_upagrahas(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :sun_based_upagrahas))

  def time_upagraha_jd(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :time_upagraha_jd))

  def all_sphutas(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :all_sphutas))

  def calculate_ashtakavarga(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :calculate_ashtakavarga))

  def calculate_bav(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :calculate_bav))

  def calculate_all_bav(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :calculate_all_bav))

  def calculate_sav(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :calculate_sav))

  def trikona_sodhana(request),
    do: Native.call_util(&Native.util_run/1, Map.put(request, :op, :trikona_sodhana))

  def ekadhipatya_sodhana(request),
    do:
      Native.call_util(&Native.util_run/1, Map.put(request, :op, :ekadhipatya_sodhana))
end
