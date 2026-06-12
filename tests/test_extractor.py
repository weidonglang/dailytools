import zipfile

import pytest

from core.extractor import ExtractionError, safe_extract_zip


def test_zip_path_traversal_is_rejected(tmp_path):
    archive = tmp_path / "unsafe.zip"
    with zipfile.ZipFile(archive, "w") as bundle:
        bundle.writestr("../outside.txt", "unsafe")
    with pytest.raises(ExtractionError):
        safe_extract_zip(archive, tmp_path / "output")
    assert not (tmp_path / "outside.txt").exists()
