package dhruv

import "testing"

func TestAmshaVariations(t *testing.T) {
	out, err := AmshaVariations(2)
	if err != nil {
		t.Fatalf("AmshaVariations(D2) failed: %v", err)
	}
	if out.AmshaCode != 2 {
		t.Fatalf("expected amsha 2, got %d", out.AmshaCode)
	}
	if out.DefaultVariationCode != 0 {
		t.Fatalf("expected default variation 0, got %d", out.DefaultVariationCode)
	}
	if len(out.Variations) != 2 {
		t.Fatalf("expected 2 D2 variations, got %d", len(out.Variations))
	}
	if out.Variations[1].Name != "cancer-leo-only" {
		t.Fatalf("unexpected D2 variation name: %q", out.Variations[1].Name)
	}
}

func TestAmshaVariationsMany(t *testing.T) {
	out, err := AmshaVariationsMany([]uint16{2, 9})
	if err != nil {
		t.Fatalf("AmshaVariationsMany failed: %v", err)
	}
	if len(out) != 2 {
		t.Fatalf("expected 2 catalogs, got %d", len(out))
	}
	if out[1].AmshaCode != 9 {
		t.Fatalf("expected second catalog to be D9, got D%d", out[1].AmshaCode)
	}
	if len(out[1].Variations) != 1 || out[1].Variations[0].VariationCode != 0 {
		t.Fatalf("unexpected D9 variation catalog: %+v", out[1].Variations)
	}
}
