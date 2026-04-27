defmodule CtaraDhruvTest do
  use ExUnit.Case

  alias CtaraDhruv.{Dasha, Engine, Ephemeris, Jyotish, Math, Panchang, Search, Tara, Time, Vedic}

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
          assert {:ok, _} = Search.sankranti(engine, %{mode: :next, at_utc: utc})
          assert {:ok, _} = Jyotish.graha_positions(engine, %{utc: utc, location: location})
          assert {:ok, _} = Jyotish.bindus(engine, %{utc: utc, location: location})

          assert {:ok, _} =
                   Dasha.hierarchy(engine, %{
                     birth_utc: utc,
                     location: location,
                     system: :vimshottari,
                     max_level: 1
                   })

          assert {:ok, level0} =
                   Dasha.level0(engine, %{
                     birth_utc: utc,
                     location: location,
                     system: :vimshottari
                   })

          assert length(level0) > 0
          first = hd(level0)

          assert {:ok, level0_entity} =
                   Dasha.level0_entity(engine, %{
                     birth_utc: utc,
                     location: location,
                     system: :vimshottari,
                     entity: first.entity
                   })

          assert level0_entity.entity.index == first.entity.index

          assert {:ok, children} =
                   Dasha.children(engine, %{
                     birth_utc: utc,
                     location: location,
                     system: :vimshottari,
                     parent: first
                   })

          assert length(children) > 0
          first_child = hd(children)

          assert {:ok, child_period} =
                   Dasha.child_period(engine, %{
                     birth_utc: utc,
                     location: location,
                     system: :vimshottari,
                     parent: first,
                     child_entity: first_child.entity
                   })

          assert child_period.entity.index == first_child.entity.index

          assert {:ok, complete_level} =
                   Dasha.complete_level(engine, %{
                     birth_utc: utc,
                     location: location,
                     system: :vimshottari,
                     parent_periods: level0,
                     child_level: :antardasha
                   })

          assert length(complete_level) >= length(children)
        end

        if File.exists?(@tara) do
          assert {:ok, _} = Engine.load_tara_catalog(engine, @tara)
          assert {:ok, _} = Tara.catalog_info(engine)
        else
          assert {:ok, _} = Tara.catalog_info(engine)
        end
    end
  end

  test "elixir engine constructor accepts omitted shared default fields" do
    if File.exists?(@spk) and File.exists?(@lsk) do
      assert {:ok, engine} =
               Engine.new(%{
                 spk_paths: [@spk],
                 lsk_path: @lsk,
                 time_policy: %{mode: :hybrid_delta_t}
               })

      assert {:ok, %{closed: true}} = Engine.close(engine)
    else
      assert true
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

          too_many_systems = List.duplicate(:vimshottari, 24)

          assert {:error, %CtaraDhruv.Error{kind: :invalid_request, message: message}} =
                   Jyotish.full_kundali(engine, %{
                     utc: utc,
                     location: location,
                     full_kundali_config: %{
                       include_dasha: true,
                       dasha_config: %{systems: too_many_systems}
                     }
                   })

          assert message =~ "systems may contain at most"
        else
          assert true
        end
    end
  end

  test "elixir jyotish wrappers accept amsha_selection and return resolved amsha union" do
    case with_engine() do
      :skip ->
        assert true

      {:ok, engine} ->
        if File.exists?(@eop) do
          assert {:ok, _} = Engine.load_eop(engine, @eop)

          location = %{latitude_deg: 28.6139, longitude_deg: 77.2090, altitude_m: 0.0}
          utc = %{year: 2015, month: 1, day: 15, hour: 6, minute: 0, second: 0.0}
          d2_variation = [%{code: 2, variation: 1}]
          d9_default = [%{code: 9}]

          assert {:ok, shadbala} =
                   Jyotish.shadbala(engine, %{
                     utc: utc,
                     location: location,
                     amsha_selection: d2_variation
                   })

          assert length(shadbala.entries) == 7

          assert {:ok, vimsopaka} =
                   Jyotish.vimsopaka(engine, %{
                     utc: utc,
                     location: location,
                     amsha_selection: d2_variation
                   })

          assert length(vimsopaka.entries) == 9

          assert {:ok, balas} =
                   Jyotish.balas(engine, %{
                     utc: utc,
                     location: location,
                     amsha_selection: d2_variation
                   })

          assert length(balas.shadbala.entries) == 7
          assert length(balas.vimsopaka.entries) == 9

          assert {:ok, avastha} =
                   Jyotish.avastha(engine, %{
                     utc: utc,
                     location: location,
                     amsha_selection: d9_default
                   })

          assert length(avastha.entries) == 9

          assert {:ok, chart} =
                   Jyotish.full_kundali(engine, %{
                     utc: utc,
                     location: location,
                     full_kundali_config: %{
                       include_amshas: true,
                       include_shadbala: true,
                       include_vimsopaka: true,
                       amsha_selection: d2_variation
                     }
                   })

          assert length(chart.amshas.charts) == 16
          assert hd(chart.amshas.charts).amsha == "d2"
          assert hd(chart.amshas.charts).variation == "cancer-leo-only"
          assert Enum.any?(chart.amshas.charts, &(&1.amsha == "d60"))
        else
          assert true
        end
    end
  end

  test "engine replaces and lists spks" do
    case with_engine() do
      :skip ->
        assert true

      {:ok, engine} ->
        assert {:ok, initial} = Engine.list_spks(engine)
        assert length(initial.spks) == 1
        assert hd(initial.spks).generation == 0

        assert {:ok, report} = Engine.replace_spks(engine, [@spk, @spk])
        assert report.generation == 1
        assert report.active_count == 2
        assert report.loaded_count == 0
        assert report.reused_count == 2

        assert {:ok, active} = Engine.list_spks(engine)
        assert length(active.spks) == 2
        assert Enum.all?(active.spks, &(&1.generation == report.generation))

        missing = Path.join(@kernel_dir, "missing.bsp")
        assert {:error, _} = Engine.replace_spks(engine, [missing])
        assert {:ok, after_failure} = Engine.list_spks(engine)
        assert hd(after_failure.spks).generation == report.generation
    end
  end

  test "elixir math exposes amsha variation catalogs" do
    assert {:ok, d2} = Math.amsha_variations(%{amsha_code: 2})
    assert d2.amsha_code == 2
    assert d2.default_variation_code == 0
    assert Enum.any?(d2.variations, &(&1.name == "cancer-leo-only" and &1.variation_code == 1))

    assert {:ok, many} = Math.amsha_variations_many(%{amsha_codes: [2, 9]})
    assert length(many.catalogs) == 2
    assert Enum.at(many.catalogs, 1).amsha_code == 9
    assert Enum.at(Enum.at(many.catalogs, 1).variations, 0).is_default == true
  end
end
