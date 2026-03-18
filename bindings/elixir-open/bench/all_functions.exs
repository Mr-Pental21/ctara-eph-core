Mix.Task.run("app.start")

alias CtaraDhruv
alias CtaraDhruv.{Dasha, Engine, Ephemeris, Jyotish, Panchang, Search, Tara, Time, Vedic}

defmodule CtaraDhruv.Bench.AllFunctions do
  @moduledoc false

  @public_modules [
    CtaraDhruv,
    CtaraDhruv.Engine,
    CtaraDhruv.Ephemeris,
    CtaraDhruv.Time,
    CtaraDhruv.Vedic,
    CtaraDhruv.Panchang,
    CtaraDhruv.Search,
    CtaraDhruv.Jyotish,
    CtaraDhruv.Dasha,
    CtaraDhruv.Tara
  ]

  def main do
    iterations = positive_env("DHRUV_BENCH_ITERATIONS", 3)
    warmup = positive_env("DHRUV_BENCH_WARMUP", 1)
    filter = System.get_env("DHRUV_BENCH_FILTER")
    output_path = System.get_env("DHRUV_BENCH_OUTPUT_PATH")

    context = build_context()

    try do
      cases = benchmark_cases(context)
      assert_full_coverage!(cases)

      selected_cases =
        cases
        |> maybe_filter(filter)

      IO.puts("ctara_dhruv Elixir wrapper benchmark")
      IO.puts("iterations=#{iterations} warmup=#{warmup} filter=#{filter || "*"}")
      IO.puts("")

      results =
        Enum.map(selected_cases, fn bench_case ->
          result = benchmark_case(bench_case, context, warmup, iterations)
          print_result(result)
          result
        end)

      print_summary(results)
      maybe_write_markdown_report(output_path, results, context, iterations, warmup, filter)
    after
      cleanup_context(context)
    end
  end

  defp benchmark_cases(context) do
    common_utc = context.common_utc
    query_utc = context.query_utc
    location = context.location
    search_jd = context.search_jd
    base_jd = context.base_jd

    [
      case_spec("CtaraDhruv.new_engine/1", [:engine_files], fn ctx ->
        {:ok, engine} = CtaraDhruv.new_engine(engine_config(ctx))
        assert_ok!(Engine.close(engine))
      end),
      case_spec("CtaraDhruv.Engine.new/1", [:engine_files], fn ctx ->
        {:ok, engine} = Engine.new(engine_config(ctx))
        assert_ok!(Engine.close(engine))
      end),
      case_spec("CtaraDhruv.Engine.close/1", [:engine_files], fn ctx ->
        {:ok, engine} = Engine.new(engine_config(ctx))
        assert_ok!(Engine.close(engine))
      end),
      case_spec("CtaraDhruv.Engine.load_config/2", [:engine_files, :config_file], fn ctx ->
        {:ok, engine} = Engine.new(engine_config(ctx))

        try do
          assert_ok!(Engine.load_config(engine, ctx.config_path))
        after
          close_quietly(engine)
        end
      end),
      case_spec("CtaraDhruv.Engine.clear_config/1", [:engine_files, :config_file], fn ctx ->
        {:ok, engine} = Engine.new(engine_config(ctx))

        try do
          assert_ok!(Engine.load_config(engine, ctx.config_path))
          assert_ok!(Engine.clear_config(engine))
        after
          close_quietly(engine)
        end
      end),
      case_spec("CtaraDhruv.Engine.load_eop/2", [:engine_files, :eop_file], fn ctx ->
        {:ok, engine} = Engine.new(engine_config(ctx))

        try do
          assert_ok!(Engine.load_eop(engine, ctx.eop))
        after
          close_quietly(engine)
        end
      end),
      case_spec("CtaraDhruv.Engine.clear_eop/1", [:engine_files, :eop_file], fn ctx ->
        {:ok, engine} = Engine.new(engine_config(ctx))

        try do
          assert_ok!(Engine.load_eop(engine, ctx.eop))
          assert_ok!(Engine.clear_eop(engine))
        after
          close_quietly(engine)
        end
      end),
      case_spec("CtaraDhruv.Engine.load_tara_catalog/2", [:engine_files, :tara_file], fn ctx ->
        {:ok, engine} = Engine.new(engine_config(ctx))

        try do
          assert_ok!(Engine.load_tara_catalog(engine, ctx.tara))
        after
          close_quietly(engine)
        end
      end),
      case_spec("CtaraDhruv.Engine.reset_tara_catalog/1", [:engine_files, :tara_file], fn ctx ->
        {:ok, engine} = Engine.new(engine_config(ctx))

        try do
          assert_ok!(Engine.load_tara_catalog(engine, ctx.tara))
          assert_ok!(Engine.reset_tara_catalog(engine))
        after
          close_quietly(engine)
        end
      end),
      case_spec("CtaraDhruv.Engine.set_time_policy/2", [:shared_engine], fn ctx ->
        assert_ok!(Engine.set_time_policy(ctx.shared_engine, %{mode: :hybrid_delta_t}))
      end),
      case_spec("CtaraDhruv.Ephemeris.query/2", [:shared_engine], fn ctx ->
        assert_ok!(
          Ephemeris.query(ctx.shared_engine, %{
            target: :mars,
            observer: :solar_system_barycenter,
            frame: 1,
            epoch_tdb_jd: base_jd
          })
        )
      end),
      case_spec("CtaraDhruv.Ephemeris.query_utc/2", [:shared_engine], fn ctx ->
        assert_ok!(
          Ephemeris.query_utc(ctx.shared_engine, %{
            target: :mars,
            observer: :solar_system_barycenter,
            frame: 1,
            utc: common_utc
          })
        )
      end),
      case_spec("CtaraDhruv.Ephemeris.query_utc_spherical/2", [:shared_engine], fn ctx ->
        assert_ok!(
          Ephemeris.query_utc_spherical(ctx.shared_engine, %{
            target: :mars,
            observer: :solar_system_barycenter,
            frame: 1,
            utc: common_utc
          })
        )
      end),
      case_spec("CtaraDhruv.Ephemeris.body_ecliptic_lon_lat/2", [:shared_engine], fn ctx ->
        assert_ok!(
          Ephemeris.body_ecliptic_lon_lat(ctx.shared_engine, %{body: :mars, jd_tdb: base_jd})
        )
      end),
      case_spec("CtaraDhruv.Ephemeris.cartesian_to_spherical/1", [], fn _ctx ->
        assert_ok!(Ephemeris.cartesian_to_spherical(%{x: 1.0, y: 0.0, z: 0.0}))
      end),
      case_spec("CtaraDhruv.Time.utc_to_jd_tdb/2", [:shared_engine], fn ctx ->
        assert_ok!(Time.utc_to_jd_tdb(ctx.shared_engine, %{utc: common_utc}))
      end),
      case_spec("CtaraDhruv.Time.jd_tdb_to_utc/2", [:shared_engine], fn ctx ->
        assert_ok!(Time.jd_tdb_to_utc(ctx.shared_engine, %{jd_tdb: base_jd}))
      end),
      case_spec("CtaraDhruv.Time.nutation/1", [], fn _ctx ->
        assert_ok!(Time.nutation(%{jd_tdb: base_jd}))
      end),
      case_spec("CtaraDhruv.Vedic.ayanamsha/2", [:shared_engine], fn ctx ->
        assert_ok!(
          Vedic.ayanamsha(ctx.shared_engine, %{
            jd_tdb: base_jd,
            system: :lahiri,
            mode: :unified
          })
        )
      end),
      case_spec("CtaraDhruv.Vedic.lunar_node/2", [:shared_engine], fn ctx ->
        assert_ok!(
          Vedic.lunar_node(ctx.shared_engine, %{
            system: :rahu,
            mode: "true",
            backend: :engine,
            jd_tdb: base_jd
          })
        )
      end),
      case_spec("CtaraDhruv.Vedic.rise_set/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(
          Vedic.rise_set(ctx.shared_engine, %{
            utc: common_utc,
            location: location,
            event: :sunrise
          })
        )
      end),
      case_spec("CtaraDhruv.Vedic.all_events/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(Vedic.all_events(ctx.shared_engine, %{utc: common_utc, location: location}))
      end),
      case_spec("CtaraDhruv.Vedic.lagna/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(Vedic.lagna(ctx.shared_engine, %{utc: common_utc, location: location}))
      end),
      case_spec("CtaraDhruv.Vedic.mc/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(Vedic.mc(ctx.shared_engine, %{utc: common_utc, location: location}))
      end),
      case_spec("CtaraDhruv.Vedic.ramc/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(Vedic.ramc(ctx.shared_engine, %{utc: common_utc, location: location}))
      end),
      case_spec("CtaraDhruv.Vedic.bhavas/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(Vedic.bhavas(ctx.shared_engine, %{utc: common_utc, location: location}))
      end),
      case_spec("CtaraDhruv.Panchang.tithi/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(Panchang.tithi(ctx.shared_engine, %{utc: common_utc}))
      end),
      case_spec("CtaraDhruv.Panchang.karana/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(Panchang.karana(ctx.shared_engine, %{utc: common_utc}))
      end),
      case_spec("CtaraDhruv.Panchang.yoga/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(Panchang.yoga(ctx.shared_engine, %{utc: common_utc}))
      end),
      case_spec("CtaraDhruv.Panchang.nakshatra/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(Panchang.nakshatra(ctx.shared_engine, %{utc: common_utc}))
      end),
      case_spec("CtaraDhruv.Panchang.vaar/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(Panchang.vaar(ctx.shared_engine, %{utc: common_utc, location: location}))
      end),
      case_spec("CtaraDhruv.Panchang.hora/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(Panchang.hora(ctx.shared_engine, %{utc: common_utc, location: location}))
      end),
      case_spec("CtaraDhruv.Panchang.ghatika/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(Panchang.ghatika(ctx.shared_engine, %{utc: common_utc, location: location}))
      end),
      case_spec("CtaraDhruv.Panchang.masa/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(Panchang.masa(ctx.shared_engine, %{utc: common_utc}))
      end),
      case_spec("CtaraDhruv.Panchang.ayana/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(Panchang.ayana(ctx.shared_engine, %{utc: common_utc}))
      end),
      case_spec("CtaraDhruv.Panchang.varsha/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(Panchang.varsha(ctx.shared_engine, %{utc: common_utc}))
      end),
      case_spec("CtaraDhruv.Panchang.daily/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(Panchang.daily(ctx.shared_engine, %{utc: common_utc, location: location}))
      end),
      case_spec("CtaraDhruv.Search.conjunction/2", [:shared_engine], fn ctx ->
        assert_ok!(
          Search.conjunction(ctx.shared_engine, %{
            mode: :next,
            body1: :sun,
            body2: :moon,
            at_jd_tdb: base_jd
          })
        )
      end),
      case_spec("CtaraDhruv.Search.grahan/2", [:shared_engine], fn ctx ->
        assert_ok!(
          Search.grahan(ctx.shared_engine, %{
            mode: :next,
            kind: :surya,
            at_jd_tdb: search_jd
          })
        )
      end),
      case_spec("CtaraDhruv.Search.lunar_phase/2", [:shared_engine], fn ctx ->
        assert_ok!(
          Search.lunar_phase(ctx.shared_engine, %{
            mode: :next,
            kind: :purnima,
            at_jd_tdb: base_jd
          })
        )
      end),
      case_spec("CtaraDhruv.Search.sankranti/2", [:shared_engine], fn ctx ->
        assert_ok!(Search.sankranti(ctx.shared_engine, %{mode: :next, at_jd_tdb: base_jd}))
      end),
      case_spec("CtaraDhruv.Search.motion/2", [:shared_engine], fn ctx ->
        assert_ok!(
          Search.motion(ctx.shared_engine, %{
            mode: :next,
            body: :mercury,
            kind: :stationary,
            at_jd_tdb: search_jd
          })
        )
      end),
      case_spec("CtaraDhruv.Jyotish.graha_longitudes/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(
          Jyotish.graha_longitudes(ctx.shared_engine, %{jd_tdb: base_jd, system: :lahiri})
        )
      end),
      case_spec("CtaraDhruv.Jyotish.graha_positions/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(
          Jyotish.graha_positions(ctx.shared_engine, %{utc: common_utc, location: location})
        )
      end),
      case_spec("CtaraDhruv.Jyotish.special_lagnas/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(
          Jyotish.special_lagnas(ctx.shared_engine, %{utc: common_utc, location: location})
        )
      end),
      case_spec("CtaraDhruv.Jyotish.arudha/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(Jyotish.arudha(ctx.shared_engine, %{utc: common_utc, location: location}))
      end),
      case_spec("CtaraDhruv.Jyotish.upagrahas/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(Jyotish.upagrahas(ctx.shared_engine, %{utc: common_utc, location: location}))
      end),
      case_spec("CtaraDhruv.Jyotish.bindus/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(Jyotish.bindus(ctx.shared_engine, %{utc: common_utc, location: location}))
      end),
      case_spec("CtaraDhruv.Jyotish.ashtakavarga/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(
          Jyotish.ashtakavarga(ctx.shared_engine, %{utc: common_utc, location: location})
        )
      end),
      case_spec("CtaraDhruv.Jyotish.drishti/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(Jyotish.drishti(ctx.shared_engine, %{utc: common_utc, location: location}))
      end),
      case_spec("CtaraDhruv.Jyotish.charakaraka/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(Jyotish.charakaraka(ctx.shared_engine, %{utc: common_utc, scheme: :eight}))
      end),
      case_spec("CtaraDhruv.Jyotish.shadbala/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(Jyotish.shadbala(ctx.shared_engine, %{utc: common_utc, location: location}))
      end),
      case_spec("CtaraDhruv.Jyotish.vimsopaka/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(
          Jyotish.vimsopaka(ctx.shared_engine, %{
            utc: common_utc,
            location: location,
            node_dignity_policy: :sign_lord_based
          })
        )
      end),
      case_spec("CtaraDhruv.Jyotish.avastha/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(Jyotish.avastha(ctx.shared_engine, %{utc: common_utc, location: location}))
      end),
      case_spec("CtaraDhruv.Jyotish.full_kundali/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(
          Jyotish.full_kundali(ctx.shared_engine, %{utc: common_utc, location: location})
        )
      end),
      case_spec("CtaraDhruv.Jyotish.amsha/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(
          Jyotish.amsha(ctx.shared_engine, %{
            utc: common_utc,
            location: location,
            amsha_requests: [%{code: 9}]
          })
        )
      end),
      case_spec("CtaraDhruv.Dasha.hierarchy/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(
          Dasha.hierarchy(ctx.shared_engine, %{
            birth_utc: common_utc,
            location: location,
            system: :vimshottari,
            max_level: 2
          })
        )
      end),
      case_spec("CtaraDhruv.Dasha.snapshot/2", [:shared_engine, :eop_file], fn ctx ->
        assert_ok!(
          Dasha.snapshot(ctx.shared_engine, %{
            birth_utc: common_utc,
            query_utc: query_utc,
            location: location,
            system: :vimshottari,
            max_level: 2
          })
        )
      end),
      case_spec("CtaraDhruv.Tara.compute/2", [:shared_engine], fn ctx ->
        assert_ok!(
          Tara.compute(ctx.shared_engine, %{
            star: "Sirius",
            jd_tdb: base_jd,
            output: :ecliptic
          })
        )
      end),
      case_spec("CtaraDhruv.Tara.catalog_info/1", [:shared_engine], fn ctx ->
        assert_ok!(Tara.catalog_info(ctx.shared_engine))
      end)
    ]
  end

  defp build_context do
    repo_root = Path.expand("../../..", __DIR__)
    kernel_dir = Path.join(repo_root, "kernels/data")
    spk = Path.join(kernel_dir, "de442s.bsp")
    lsk = Path.join(kernel_dir, "naif0012.tls")
    eop = Path.join(kernel_dir, "finals2000A.all")
    tara = Path.join(kernel_dir, "hgca_tara.json")
    config_path = write_temp_config()

    context = %{
      repo_root: repo_root,
      spk: spk,
      lsk: lsk,
      eop: eop,
      tara: tara,
      config_path: config_path,
      base_jd: 2_457_174.75,
      search_jd: 2_457_204.75,
      common_utc: %{year: 2015, month: 6, day: 1, hour: 6, minute: 0, second: 0.0},
      query_utc: %{year: 2015, month: 7, day: 1, hour: 6, minute: 0, second: 0.0},
      location: %{latitude_deg: 28.6139, longitude_deg: 77.2090, altitude_m: 0.0}
    }

    Map.put(context, :shared_engine, maybe_build_shared_engine(context))
  end

  defp maybe_build_shared_engine(context) do
    if available?(context, :engine_files) do
      {:ok, engine} = Engine.new(engine_config(context))

      try do
        assert_ok!(Engine.load_config(engine, context.config_path))

        if available?(context, :eop_file) do
          assert_ok!(Engine.load_eop(engine, context.eop))
        end

        if available?(context, :tara_file) do
          assert_ok!(Engine.load_tara_catalog(engine, context.tara))
        end

        assert_ok!(Engine.set_time_policy(engine, %{mode: :hybrid_delta_t}))
        engine
      rescue
        error ->
          close_quietly(engine)
          reraise error, __STACKTRACE__
      end
    else
      nil
    end
  end

  defp write_temp_config do
    path =
      Path.join(
        System.tmp_dir!(),
        "ctara_dhruv_elixir_bench_config_#{System.unique_integer([:positive])}.toml"
      )

    File.write!(
      path,
      """
      version = 1

      [operations.conjunction]
      step_size_days = 1.0
      """
    )

    path
  end

  defp benchmark_case(bench_case, context, warmup, iterations) do
    missing = missing_requirements(bench_case.requires, context)

    if missing != [] do
      %{name: bench_case.name, status: :skipped, reason: Enum.join(missing, ", ")}
    else
      Enum.each(1..warmup, fn _ -> bench_case.run.(context) end)

      times =
        for _ <- 1..iterations do
          {micros, _value} = :timer.tc(fn -> bench_case.run.(context) end)
          micros
        end

      %{
        name: bench_case.name,
        status: :ok,
        count: iterations,
        avg_ms: Enum.sum(times) / length(times) / 1_000.0,
        min_ms: Enum.min(times) / 1_000.0,
        max_ms: Enum.max(times) / 1_000.0
      }
    end
  rescue
    error ->
      %{name: bench_case.name, status: :failed, reason: Exception.message(error)}
  end

  defp maybe_filter(cases, nil), do: cases

  defp maybe_filter(cases, pattern) do
    regex = Regex.compile!(pattern)
    Enum.filter(cases, fn bench_case -> Regex.match?(regex, bench_case.name) end)
  end

  defp missing_requirements(requires, context) do
    Enum.reject(requires, &available?(context, &1))
    |> Enum.map(&Atom.to_string/1)
  end

  defp available?(context, :engine_files) do
    File.exists?(context.spk) and File.exists?(context.lsk)
  end

  defp available?(context, :eop_file), do: File.exists?(context.eop)
  defp available?(context, :tara_file), do: File.exists?(context.tara)
  defp available?(context, :config_file), do: File.exists?(context.config_path)
  defp available?(context, :shared_engine), do: not is_nil(context.shared_engine)

  defp engine_config(context) do
    %{
      spk_paths: [context.spk],
      lsk_path: context.lsk,
      cache_capacity: 64,
      strict_validation: false
    }
  end

  defp assert_full_coverage!(cases) do
    expected =
      @public_modules
      |> Enum.flat_map(&public_functions/1)
      |> MapSet.new()

    actual =
      cases
      |> Enum.map(& &1.name)
      |> MapSet.new()

    missing = MapSet.difference(expected, actual) |> MapSet.to_list() |> Enum.sort()
    extra = MapSet.difference(actual, expected) |> MapSet.to_list() |> Enum.sort()

    cond do
      missing != [] ->
        raise "benchmark coverage is missing public functions: #{Enum.join(missing, ", ")}"

      extra != [] ->
        raise "benchmark coverage has unexpected function labels: #{Enum.join(extra, ", ")}"

      true ->
        :ok
    end
  end

  defp public_functions(module) do
    module.__info__(:functions)
    |> Enum.reject(fn {name, _arity} -> name in [:__info__, :__struct__, :module_info] end)
    |> Enum.map(fn {name, arity} -> "#{inspect(module)}.#{name}/#{arity}" end)
  end

  defp case_spec(name, requires, run), do: %{name: name, requires: requires, run: run}

  defp close_quietly(engine) do
    _ = Engine.close(engine)
    :ok
  end

  defp assert_ok!({:ok, value}), do: value

  defp assert_ok!({:error, error}) do
    raise "#{inspect(error.__struct__)} #{inspect(Map.from_struct(error))}"
  end

  defp print_result(%{status: :ok} = result) do
    IO.puts(
      String.pad_trailing(result.name, 42) <>
        " ok    avg=#{format_ms(result.avg_ms)} min=#{format_ms(result.min_ms)} max=#{format_ms(result.max_ms)}"
    )
  end

  defp print_result(%{status: :skipped} = result) do
    IO.puts(String.pad_trailing(result.name, 42) <> " skip  #{result.reason}")
  end

  defp print_result(%{status: :failed} = result) do
    IO.puts(String.pad_trailing(result.name, 42) <> " fail  #{result.reason}")
  end

  defp print_summary(results) do
    ok = Enum.filter(results, &(&1.status == :ok))
    skipped = Enum.count(results, &(&1.status == :skipped))
    failed = Enum.count(results, &(&1.status == :failed))

    IO.puts("")

    IO.puts(
      "summary: ok=#{length(ok)} skipped=#{skipped} failed=#{failed} total=#{length(results)}"
    )

    if ok != [] do
      slowest =
        ok
        |> Enum.sort_by(& &1.avg_ms, :desc)
        |> Enum.take(5)

      IO.puts("slowest:")

      Enum.each(slowest, fn result ->
        IO.puts("  #{result.name} avg=#{format_ms(result.avg_ms)}")
      end)
    end
  end

  defp maybe_write_markdown_report(nil, _results, _context, _iterations, _warmup, _filter),
    do: :ok

  defp maybe_write_markdown_report(output_path, results, context, iterations, warmup, filter) do
    File.mkdir_p!(Path.dirname(output_path))
    File.write!(output_path, markdown_report(results, context, iterations, warmup, filter))
    IO.puts("")
    IO.puts("report: #{output_path}")
  end

  defp markdown_report(results, context, iterations, warmup, filter) do
    ok =
      results
      |> Enum.filter(&(&1.status == :ok))
      |> Enum.sort_by(& &1.avg_ms, :desc)

    skipped =
      results
      |> Enum.filter(&(&1.status == :skipped))
      |> Enum.sort_by(& &1.name)

    failed =
      results
      |> Enum.filter(&(&1.status == :failed))
      |> Enum.sort_by(& &1.name)

    [
      "# Elixir Wrapper Benchmarks",
      "",
      "Generated on #{Date.utc_today()} from `bindings/elixir-open/bench/all_functions.exs`.",
      "",
      "## Environment",
      "",
      "- Elixir: #{System.version()}",
      "- OTP: #{List.to_string(:erlang.system_info(:otp_release))}",
      "- Iterations per function: #{iterations}",
      "- Warmup runs per function: #{warmup}",
      "- Filter: `#{filter || "*"}`",
      "- SPK: `#{context.spk}`",
      "- LSK: `#{context.lsk}`",
      "- EOP: `#{context.eop}`",
      "- Tara catalog: `#{context.tara}`",
      "- Benchmark UTC fixture: `#{inspect(context.common_utc)}`",
      "- Benchmark JD fixture: `#{context.base_jd}`",
      "",
      "## Summary",
      "",
      "- Successful benchmarks: #{length(ok)}",
      "- Skipped benchmarks: #{length(skipped)}",
      "- Failed benchmarks: #{length(failed)}",
      "",
      "## Timings (Sorted By Average Descending)",
      "",
      "| Function | Avg (ms) | Min (ms) | Max (ms) | Iterations |",
      "|---|---:|---:|---:|---:|"
    ]
    |> Kernel.++(
      Enum.map(ok, fn result ->
        "| `#{result.name}` | #{fixed_ms(result.avg_ms)} | #{fixed_ms(result.min_ms)} | #{fixed_ms(result.max_ms)} | #{result.count} |"
      end)
    )
    |> Kernel.++(
      markdown_optional_section("Skipped", skipped, fn result ->
        "- `#{result.name}`: #{result.reason}"
      end)
    )
    |> Kernel.++(
      markdown_optional_section("Failed", failed, fn result ->
        "- `#{result.name}`: #{result.reason}"
      end)
    )
    |> Enum.join("\n")
    |> Kernel.<>("\n")
  end

  defp markdown_optional_section(_title, [], _formatter), do: []

  defp markdown_optional_section(title, results, formatter) do
    ["", "## #{title}", ""] ++ Enum.map(results, formatter)
  end

  defp cleanup_context(context) do
    if context.shared_engine do
      close_quietly(context.shared_engine)
    end

    File.rm(context.config_path)
  end

  defp format_ms(value), do: :erlang.float_to_binary(value, decimals: 3) <> "ms"
  defp fixed_ms(value), do: :erlang.float_to_binary(value, decimals: 3)

  defp positive_env(name, default) do
    case System.get_env(name) do
      nil ->
        default

      value ->
        case Integer.parse(value) do
          {parsed, ""} when parsed > 0 -> parsed
          _ -> raise "invalid #{name}=#{inspect(value)}"
        end
    end
  end
end

CtaraDhruv.Bench.AllFunctions.main()
