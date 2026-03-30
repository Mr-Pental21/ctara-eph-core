defmodule CtaraDhruvTest do
  use ExUnit.Case

  alias CtaraDhruv.{Dasha, Engine, Ephemeris, Jyotish, Panchang, Search, Tara, Time, Vedic}

  @repo_root Path.expand("../../..", __DIR__)
  @kernel_dir Path.join(@repo_root, "kernels/data")
  @spk Path.join(@kernel_dir, "de442s.bsp")
  @lsk Path.join(@kernel_dir, "naif0012.tls")
  @eop Path.join(@kernel_dir, "finals2000A.all")
  @tara Path.join(@kernel_dir, "hgca_tara.json")

  defp with_engine do
    if File.exists?(@spk) and File.exists?(@lsk) do
      {:ok, engine} =
        Engine.new(%{
          spk_paths: [@spk],
          lsk_path: @lsk,
          cache_capacity: 64,
          strict_validation: false,
          time_policy: %{mode: :hybrid_delta_t}
        })

      on_exit(fn -> Engine.close(engine) end)
      {:ok, engine}
    else
      :skip
    end
  end

  test "engine lifecycle and native families smoke" do
    case with_engine() do
      :skip ->
        assert true

      {:ok, engine} ->
        assert {:ok, _} = Ephemeris.cartesian_to_spherical(%{x: 1.0, y: 0.0, z: 0.0})
        assert {:ok, _} = Time.nutation(%{jd_tdb: 2_451_545.0})

        assert {:ok, _} =
                 Ephemeris.query(engine, %{
                   target: 499,
                   observer: 0,
                   frame: 1,
                   epoch_tdb_jd: 2_451_545.0
                 })

        assert {:ok, %{diagnostics: diagnostics}} =
                 Time.utc_to_jd_tdb(engine, %{
                   utc: %{year: 2015, month: 1, day: 1, hour: 12, minute: 0, second: 0.0}
                 })

        assert is_map(diagnostics)

        if File.exists?(@eop) do
          assert {:ok, _} = Engine.load_eop(engine, @eop)
          location = %{latitude_deg: 28.6139, longitude_deg: 77.2090, altitude_m: 0.0}
          utc = %{year: 2015, month: 1, day: 15, hour: 6, minute: 0, second: 0.0}

          assert {:ok, _} =
                   Vedic.ayanamsha(engine, %{
                     jd_tdb: 2_460_311.0,
                     system: :lahiri,
                     use_nutation: false
                   })

          assert {:ok, _} =
                   Vedic.rise_set(engine, %{utc: utc, location: location, event: :sunrise})

          assert {:ok, _} = Panchang.tithi(engine, %{utc: utc})
          assert {:ok, _} = Search.sankranti(engine, %{mode: :next, at_jd_tdb: 2_451_545.0})
          assert {:ok, _} = Jyotish.graha_positions(engine, %{utc: utc, location: location})
          assert {:ok, _} = Jyotish.bindus(engine, %{utc: utc, location: location})

          assert {:ok, _} =
                   Dasha.hierarchy(engine, %{
                     birth_utc: utc,
                     location: location,
                     system: :vimshottari,
                     max_level: 1
                   })
        end

        if File.exists?(@tara) do
          assert {:ok, _} = Engine.load_tara_catalog(engine, @tara)
          assert {:ok, _} = Tara.catalog_info(engine)
        else
          assert {:ok, _} = Tara.catalog_info(engine)
        end
    end
  end

  test "elixir config loading supports typed request and defaults mode" do
    case with_engine() do
      :skip ->
        assert true

      {:ok, engine} ->
        dir =
          Path.join(System.tmp_dir!(), "dhruv-config-#{System.unique_integer([:positive])}")

        config_path = Path.join(dir, "config.toml")
        File.mkdir_p!(dir)
        File.write!(config_path, "version = 1\n")

        on_exit(fn ->
          File.rm_rf(dir)
        end)

        loaded_recommended =
          Engine.load_config(engine, %{path: config_path, defaults_mode: :recommended})

        assert match?({:ok, %{loaded: true}}, loaded_recommended)

        assert {:ok, %{cleared: true}} = Engine.clear_config(engine)

        loaded_explicit = Engine.load_config(engine, %{path: config_path, defaults_mode: :none})
        assert match?({:ok, %{loaded: true}}, loaded_explicit)
    end
  end

  test "elixir wrapper exposes sidereal bhavas and full_kundali defaults" do
    case with_engine() do
      :skip ->
        assert true

      {:ok, engine} ->
        if File.exists?(@eop) do
          assert {:ok, _} = Engine.load_eop(engine, @eop)

          location = %{latitude_deg: 28.6139, longitude_deg: 77.2090, altitude_m: 0.0}
          utc = %{year: 2015, month: 1, day: 15, hour: 6, minute: 0, second: 0.0}
          request = %{utc: utc, location: location}
          sidereal = %{ayanamsha_system: :lahiri, use_nutation: false}

          assert {:ok, %{longitude_deg: tropical_lagna}} = Vedic.lagna(engine, request)
          assert {:ok, %{longitude_deg: sidereal_lagna}} = Vedic.lagna(engine, request, sidereal)
          assert abs(tropical_lagna - sidereal_lagna) > 0.1

          assert {:ok, %{longitude_deg: sidereal_mc}} = Vedic.mc(engine, request, sidereal)

          assert {:ok, bhavas} = Vedic.bhavas(engine, request, sidereal)
          assert length(bhavas.bhavas) == 12
          assert_in_delta bhavas.lagna_deg, sidereal_lagna, 1.0e-6
          assert_in_delta bhavas.mc_deg, sidereal_mc, 1.0e-6

          assert {:ok, chart} = Jyotish.full_kundali(engine, request, sidereal)
          assert is_map(chart.graha_positions)
          assert is_map(chart.graha_positions.lagna)
          assert is_float(chart.graha_positions.lagna.sidereal_longitude)
          assert is_map(chart.bhava_cusps)
          assert_in_delta chart.bhava_cusps.lagna_deg, sidereal_lagna, 1.0e-6
          assert_in_delta chart.bhava_cusps.mc_deg, sidereal_mc, 1.0e-6
        else
          assert true
        end
    end
  end
end
