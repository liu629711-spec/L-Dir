// ==== FUN_0045b114 @ 0045b114 ====

undefined8 FUN_0045b114(longlong param_1,undefined8 param_2,ushort param_3)

{
  longlong *plVar1;
  ushort *puVar2;
  ushort *puVar3;
  uint uVar4;
  longlong local_28 [3];
  
  uVar4 = 0x80c0;
  do {
    puVar2 = *(ushort **)(param_1 + 0x90);
    puVar3 = *(ushort **)(param_1 + 0x88);
    if (puVar2 != (ushort *)0x0) {
      for (; *puVar3 != 0xffff; puVar3 = puVar3 + 2) {
        if (uVar4 == *puVar3) {
          if (((*puVar2 >> 9 & 1) != 0) || ((*puVar2 >> 10 & 1) != 0)) {
            *puVar2 = (puVar3[1] | *puVar2) & 63999;
          }
          if ((*puVar2 & puVar3[1]) != 0) {
            *(ushort *)(param_1 + 0x98) = *(ushort *)(param_1 + 0x98) | puVar3[1];
          }
          break;
        }
        puVar2 = puVar2 + 8;
      }
    }
    if (uVar4 == param_3) {
      puVar2 = *(ushort **)(param_1 + 0x90);
      puVar3 = *(ushort **)(param_1 + 0x88);
      if (puVar2 != (ushort *)0x0) {
        for (; *puVar3 != 0xffff; puVar3 = puVar3 + 2) {
          if (uVar4 == *puVar3) {
            if ((*puVar2 >> 9 & 1) == 0) {
              *puVar2 = (puVar3[1] | *puVar2) & 0xfbff | 0x200;
            }
            if ((*puVar2 & puVar3[1]) != 0) {
              *(ushort *)(param_1 + 0x98) = *(ushort *)(param_1 + 0x98) | puVar3[1];
            }
            break;
          }
          puVar2 = puVar2 + 8;
        }
      }
    }
    uVar4 = uVar4 + 1;
    if (0x80cb < (int)uVar4) {
      plVar1 = FUN_00407e50(local_28,L"Als");
      FUN_004554a0(param_1,plVar1,(ulonglong)(param_3 - 0x80c0));
      return 0;
    }
  } while( true );
}


