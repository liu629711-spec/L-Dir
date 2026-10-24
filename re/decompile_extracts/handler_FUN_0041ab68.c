// ==== FUN_0041ab68 @ 0041ab68 ====

LRESULT FUN_0041ab68(longlong param_1,uint param_2,WPARAM param_3,HWND param_4,undefined4 *param_5)

{
  HWND pHVar1;
  LRESULT LVar2;
  uint nIDDlgItem;
  
  pHVar1 = param_4;
  if (param_2 < 0x4f) {
    if (param_2 == 0x4e) {
      pHVar1 = *(HWND *)param_4;
    }
    else if (param_2 == 0x2b) {
      if (param_3 == 0) goto LAB_0041ac17;
      pHVar1 = *(HWND *)(param_4 + 6);
    }
    else {
      if ((param_2 != 0x2c) && (param_2 != 0x2d)) {
        if (param_2 < 0x2e) goto LAB_0041ac17;
        if (param_2 < 0x30) goto LAB_0041ac12;
        if (param_2 != 0x39) goto LAB_0041ac17;
      }
      if (param_3 == 0) goto LAB_0041ac17;
      nIDDlgItem = param_4[1].unused;
LAB_0041aba7:
      pHVar1 = GetDlgItem(*(HWND *)(param_1 + 8),nIDDlgItem);
    }
  }
  else if (param_2 == 0x111) {
    if (param_4 == (HWND)0x0) goto LAB_0041ac17;
  }
  else {
    if (param_2 < 0x114) goto LAB_0041ac17;
    if (0x115 < param_2) {
      if (param_2 < 0x132) goto LAB_0041ac17;
      if (0x138 < param_2) {
        if (param_2 != 0x210) goto LAB_0041ac17;
        if (1 < ((uint)param_3 & 0xffff) - 1) {
          nIDDlgItem = (uint)(param_3 >> 0x10) & 0xffff;
          goto LAB_0041aba7;
        }
      }
    }
  }
LAB_0041ac12:
  if (pHVar1 != (HWND)0x0) {
    LVar2 = SendMessageW(pHVar1,param_2 + 0x2000,param_3,(LPARAM)param_4);
    return LVar2;
  }
LAB_0041ac17:
  *param_5 = 0;
  return 1;
}


